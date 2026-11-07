# Design

## Interfaces

| Concern | Choice | Why |
|---|---|---|
| Engines | UCI | The standard. Stockfish and `lc0` both speak it. |
| Agents | ACP | One client for every agent, and the agent's own login. |
| Games | PGN and FEN, with `[%csl]`, `[%cal]`, `[%eval]`, `[%clk]` | Saved games and their analysis open in other tools. |
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
is per turn. `AgentKind::reports_running_total` accounts for that. Codex, OpenCode and Pi are not
measured yet; they bring their own prompts and tools, so expect far more than Claude.

Tutoring adds about 100 to 300 tokens per turn: three turns took 937, 1,106 and 1,368 with feedback
on each move.

Reviewing a game of 8 half-moves with `sonnet` took 1,561 tokens in one call. Longer games cost
more: the whole PGN goes in, and a comment comes back for each of the player's moves.

## Strength labels

Stockfish's `UCI_Elo` is on an engine scale, anchored to CCRL 40/4 and calibrated at 60 s + 0.6 s, so
it gets about 2 s a move and shows as engine Elo. There's no reliable conversion to human ratings.

Maia plays above its target and in a narrower band: community figures put the 1100, 1500 and 1900
networks at about 1450 to 1500, 1600 and 1700 on Lichess. `setup::maia_strength` shows that instead
of the network's name. It's a rough fit to a few reports, not a measurement.

## Rules for spending tokens

- All chess logic is local: legality, game end, analysis, likelihood.
- One session per game, so that the provider's prompt cache serves everything said so far.
- A turn is the last move and the FEN. The FEN keeps takebacks right without a new session.
- Legal moves are listed only after an illegal reply, once. Then the game pauses.
- Forced moves skip the agent.
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

## Not built yet

- Opening books (Polyglot).
- Asking an agent about a single position.
- Code plugins: plugins are data only.
