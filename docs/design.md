# Design

## Interfaces

| Concern | Choice | Why |
|---|---|---|
| Engines | UCI | The standard. Stockfish and `lc0` both speak it. |
| Agents | ACP | One client for every agent, and the agent's own login. |
| Games | PGN and FEN, with `[%csl]`, `[%cal]`, `[%eval]`, `[%clk]`, and Chessdesk's own `[%wdl]`, `[%better]` and `Chessdesk…` tags | Saved games and their analysis open in other tools, which skip what they don't know. |
| Plugins | Folders with a TOML manifest and plain files | Installing is copying a folder. |
| Translations | Project Fluent | Plurals for any language, and any message can be left out. |

MCP was rejected for play: tool schemas and extra round trips would cost more on every move than
putting the position in the prompt.

## What agents cost

Measured on 2026-11-07 with `claude-haiku-5-5` and Copilot's default model, from an empty folder,
in tokens per turn:

| Setup | First turn | Later turns |
|---|---|---|
| `claude -p`, no tools, own prompt | 852 | not measured |
| ACP adapter, defaults | 28,473 | 30,481 |
| ACP adapter, own prompt, no tools or settings | 4,473 | 4,574 |
| ACP, also no plugins, skills or MCP (what Chessdesk sends) | 851 | 967, then 1,088, about 90% cached |
| Copilot, no tools or custom instructions | 11,660 | about 11,750, nearly all cached |

So ACP costs Claude nothing over the direct CLI, once `plugins`, `skills` and `strictMcpConfig` are
set in the adapter's `_meta`. Those are specific to the adapter, not part of ACP; `systemPrompt`
and usage reporting sit behind the ACP crate's unstable features. Check these after updating an
adapter: `agent_smoke` prints tokens per turn.

Agents that don't advertise replacing the system prompt get Chessdesk's rules in the first message
instead, and keep their own prompt, as Copilot does. Copilot's usage is a running total where Claude's
is per turn. `AgentKind::reports_running_total` accounts for that. Codex is not measured yet; it
brings its own prompt and tools, so expect far more than Claude.

Pi's adapter, `pi-acp`, starts `pi` with fixed arguments and takes no prompt over ACP, so by default
Pi plays with its coding prompt, its tools (bash, edit, sub-agents, `codemode`), the user's
`AGENTS.md`, extensions and MCP servers, and runs tools without asking. The adapter reads the
program from `PI_ACP_PI_COMMAND`, so Chessdesk writes a launcher next to the agent folder that starts
`pi --no-tools --no-extensions --no-mcp --no-skills --no-context-files --no-prompt-templates
--system-prompt <file>` (`pi.rs`). Its thinking level is an ACP setting in the `thought_level`
category, which Chessdesk sets to the effort for any agent that offers it. Measured on 2026-11-08
with DeepSeek V4.1 Flash on low: Pi's own part of the prompt is the working folder, 140
characters. The first turns took 700 to 800 tokens; 26 turns took 125,000, mostly reasoning, for
$0.036. `pi-acp` reports no usage over ACP, so games show tokens as not reported. On one turn the
model thought for 5,300 tokens and wrote no answer, so an empty reply gets the retry too.

OpenCode doesn't replace its prompt over ACP either, but takes a config in `OPENCODE_CONFIG_CONTENT`:
Chessdesk defines an agent there with its own prompt and every permission denied, which drops all
tools, disables the title agent, and sets `OPENCODE_DISABLE_CLAUDE_CODE`, without which OpenCode
reads `~/.claude/CLAUDE.md` and Claude skills into the prompt. OpenCode's defaults also let its
tools run without a permission request, so refusing requests did nothing. Measured on 2026-11-08
with OpenCode 1.19.4 against a local endpoint that logs requests, per request:

| Setup | System prompt | Tools | About |
|---|---|---|---|
| Defaults (`build` agent) | 30,104 characters | 10 | 13,000 tokens |
| Own agent, permissions denied | 5,422 characters | 0 | 1,450 tokens |
| Also `OPENCODE_DISABLE_CLAUDE_CODE` (what Chessdesk sends) | 355 characters, plus Chessdesk's prompt | 0 | 160 tokens |

Tutoring adds about 100 to 300 tokens per turn: three turns took 937, 1,106 and 1,368 with feedback
on each move. In tutoring, a turn also carries Stockfish's verdict on the player's move, as
`Stockfish: a mistake, better was g4.`, and the agent is asked only once that's in: one live
evaluation, 150 ms. Judging the player's moves by itself, a model praised blunders and found
threats that weren't there. Comments hint at ideas and give the player's next move away only when the
instructions ask to be taught a line or told what to play: spoiling it is the player's call. Then
the agent names that one move as `[%next SAN]` and Chessdesk draws its arrow in blue, as a hint's,
if it's legal for the player. Left to draw it themselves, models marked their own move instead, or
the whole plan at once; drawn green, it was one arrow among the agent's own ideas, and the lesson
test's learner followed the wrong one.

Reviewing a game of 8 half-moves with `sonnet` took 1,561 tokens in one call. Longer games cost
more: the whole PGN goes in, and a comment comes back for each of the player's moves.

## Models

Chessdesk keeps no list of models. An agent names its models and its default when a session
starts, as the ACP session setting in the `model` category; New Game asks for that once per run
(opening a session costs no tokens) and sets the chosen model with `session/set_config_option`.
Measured on 2026-11-08: Claude Code offers 13, OpenCode 44, Pi several hundred through
OpenRouter. More than five get a list to type in.

Copilot offers none over ACP. Its default comes from `~/.copilot/settings.json` (`auto` without
one), and a model typed in New Game goes to it as `--model`.

## Strength labels

Stockfish's `UCI_Elo` is on an engine scale, anchored to CCRL 40/4 and calibrated at 60 s + 0.6 s, so
it gets about 2 s a move and shows as engine Elo. There's no reliable conversion to human ratings.

Maia plays above its target and in a narrower band: community figures put the 1100, 1500 and 1900
networks at about 1450 to 1500, 1600 and 1700 on Lichess. `setup::maia_strength` shows that instead
of the network's name. It's a rough fit to a few reports, not a measurement.

## Rules for spending tokens

- All chess logic is local: legality, game end, analysis, likelihood.
- One session per game, so that the provider's prompt cache serves everything said so far.
- A turn is the last move, where every piece stands, and the FEN. The FEN keeps takebacks right
  without a new session. The piece list, about 60 tokens, is there because models read the FEN
  badly: every blunder traced in the 2.6.0 games came from a piece the model had lost track of, a
  rook on an open file or a guarded square, and Opus on low announced a fork that wasn't one.
- A lesson the player asks for is theirs to play: the agent names their next move each turn as
  `[%next SAN]`, and plays the replies that let it work. Asked to teach a fast mate, GLM 5.3 Flash
  set out to mate the player instead, until the prompt said a mate to learn is one they give. It
  also wrote markup inside its sentences ("Your first move: [%next e4]"), which left holes once
  the commands were taken out, so the prompt shows markup after the comment.
- When the player moves first and tutoring is on, the agent gets one extra call before that move,
  for a comment and markings on the start: it would otherwise say nothing until it had moved. The
  call also opens the session, so its first move is an ordinary turn. The comment is kept in the
  PGN tag `ChessdeskIntro`, since the comment before the first move holds the player's notes.
- Legal moves are listed only after an illegal reply, once, with why the move it tried can't be
  played: "Rxe5: your pawn on e6 blocks your rook on e8." Then the game pauses. In the GLM 5.3
  Flash game of 2026-11-09 every illegal try came from a misread board, a pawn in the way or a
  bishop traded long before, and with the list alone it picked the first capture it saw and lost
  its queen. The reason only puts its reading right: listing what attacks and defends what on
  every turn would play the tactics for it, and a weak model would play far above itself. Told
  why, the model wrote about its try in the next comment, which the player never saw, until the
  retry said plainly to leave it out: in two games of 37 and 16 moves, 15 and 7 tries, all
  answered with a legal move but one.
- Forced moves skip the agent, unless it coaches: the move that forced it is often the player's worst.
- A review is one call, covering each of the player's moves.

## Threads

`chessdesk-players` blocks. The app calls it from GPUI's background executor. An ACP session has its
own thread, since the protocol crate is async and needs no runtime beyond `block_on`.

Several games can be open at once, each a `Session`. Answers carry their session's id and `epoch`,
so a move lands in the game that asked for it whichever is on show, and one that a takeback has
overtaken is dropped.

## Windows gotchas

A press on anything with `track_focus` counts as handled, and GPUI then skips its bookkeeping for
the window controls, which silently stop working. So focus is tracked below the title bar, never on
the root. Overlays stay below it too, so that the window can always be moved and closed.

Engines are console programs, and a windowed app has no console to share, so each would open one.
`uci::hide_console` starts them with `CREATE_NO_WINDOW`; the ACP crate does the same for agents.

## The likelihood

Stockfish's `wdl` models engine self-play at 60+0.6s, not people: the start position reads about
White 8%, draw 92%, Black 0%. As percentages that's misleading, so the bar and the graph show White's
expected score instead, with a draw as half a point: the start reads 54 to 46 and a lost position
goes to one end. The numbers are kept as they come, turned to White's side. A bound (`lowerbound`,
`upperbound`) only replaces nothing, since a search often ends on one.

The start's own numbers are kept in the `ChessdeskStartWdl` tag, as no move carries them, for grading
the first move. Before any move a normal game shows the middle all the same; a set-up position shows
its own numbers, or an empty bar without them.

## Analysis

A game that ends gets Stockfish's analysis at once, 300 ms a position, if Stockfish is there; the
first start always downloads it. It grades every move, marks the mistakes and records each side's
accuracy in `ChessdeskWhiteAccuracy` and `ChessdeskBlackAccuracy`. Those tags mark the game as
analysed, so Stockfish isn't offered for it again. A grade replaces tutoring's: that one came from a
150 ms look.

Accuracy follows Lichess's curve over the expected score a move gave away, with win percentage as
the expected score: `103.1668 · e^(−0.04354 · lost%) − 3.1669`, clamped to 0 to 100, and a side's
accuracy is the plain mean over its moves. Lichess weights by volatility and mixes in a harmonic
mean, so the numbers are close to its but not the same.

An agent's review ends with a `Summary {…}` line, kept on one line in `ChessdeskSummary` and left
out of the next review's PGN.

## Not built yet

- Opening books (Polyglot).
- Asking an agent about a single position.
- Code plugins: plugins are data only.
