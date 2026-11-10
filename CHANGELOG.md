# Changelog

Notable changes per version. The format follows [Keep a Changelog](https://keepachangelog.com):
minor versions bring features, patch versions only fixes.

## 2.8.0 - 2026-11-09

### Added

- Pick the model each agent reviews games and gives hints with, under Settings, Agents. Until now
  they used whatever the agent's own default was, which could be any model. Claude Code starts on
  Sonnet
- The welcome asks for those models in a third step, once an agent is set up

### Fixed

- The reasoning effort lists only the levels the model offers, in New Game and during a game,
  and none for a model without a choice. A level the model lacked, such as medium on GLM 5.3
  Flash in Pi, was skipped, so the game ran on the agent's own default
- Agents lose track of the board less: each turn tells them their own last move and yours with
  the square each piece came from and what it took, and marks their side in the piece list.
  Replaying a lesson where GLM 5.3 Flash moved a knight it no longer had, its first try was
  illegal half as often. A move played for the agent because it was forced now reaches it too
- An agent's illegal move, written first as asked, goes to the retry even when its comment
  names a legal one, which could otherwise be played in its place
- In a lesson, a next move the agent names that you can't play gets the agent to answer again,
  told why. It used to be dropped, leaving a comment that told you to play a move with no
  arrow, in up to a third of a lesson's turns
- Coaching comments no longer mention an engine, as in "the engine agrees"
- Agents' moves are read when written with the square they came from, such as Bf8-g7, as the
  turns now spell them. A model that copied that ended up with no move read, and the game paused
- A move two of the agent's pieces can make, such as Rd8 with rooks on a8 and e8, is retried
  with both named and how to write each. The agent was told its rook was pinned, which it wasn't

## 2.7.1 - 2026-11-09

### Fixed

- When an agent tries an illegal move, it's told why, such as a pawn of its own in the way or a
  bishop it no longer has, along with the legal moves. Given only the list, it misread the board
  again and grabbed the first capture, which could cost it its queen

## 2.7.0 - 2026-11-09

### Added

- When a game ends, Stockfish goes over it straight away, and the moves open beside it. Its marks
  stay with the game, so Analyze no longer offers Stockfish for a game it has been over
- The game panel shows how accurately you and your opponent played, worked out by that analysis
- Moves Stockfish found fault with carry Chessdesk's mark in the move list; hover it for why
- An agent's review ends with a short summary of the game, kept with it and shown with the result
  on the final position
- Opening a finished game opens its moves

### Changed

- The first start always downloads Stockfish: analysis, hints and the likelihood need it

### Fixed

- A review that put several moves on one line gave all their comments to the first move; each
  move now gets its own

## 2.6.1 - 2026-11-08

### Changed

- The move an agent tells you to play next is drawn blue, like a hint, so it stands out from the
  green arrows of its own ideas
- Pi's card in New Game says it has free models, as OpenCode's does: OpenRouter's, which its model
  list shows under Free, above the rest

### Fixed

- Agents are told where every piece stands on each move, not only the FEN, so they lose track of
  pieces less: most of their blunders and false claims came from misreading the board
- Ask an agent to teach you a mate or a trap and it teaches you to give it, naming your move each
  turn and playing along, instead of setting out to mate you
- Moves an agent named in its comment no longer vanish from it, leaving gaps such as "Your first
  move: ."
- When the player's move leaves the agent only one reply, a coaching agent still comments: that
  move was often the player's worst, and went unremarked
- A reply whose only move sits in `[%next]`, where the player's move goes, or is written as "e5's",
  is played instead of failing the game
- In tutoring, an agent hears Stockfish's verdict on each of your moves and the better move after
  a mistake, so it no longer praises blunders or warns of threats that aren't there. It waits the
  fraction of a second Stockfish takes, and plays on without it if Stockfish isn't installed
- OpenCode gets Chessdesk's short prompt instead of its own coding one, and no tools: a turn cost
  about 13,000 tokens of its setup, now about 160. It could also run commands without asking, and
  a hint once downloaded Stockfish to answer
- OpenCode no longer reads your `~/.claude/CLAUDE.md` and Claude skills into every game, and no
  longer spends a call per game on a title
- After an illegal move you never saw, an agent's comment no longer apologises for it or thanks you
  for the list of legal moves
- Pi plays with Chessdesk's prompt and no tools, extensions, MCP servers or `AGENTS.md`: it came
  with its whole coding setup, and could run commands without asking
- Pi thinks as hard as the effort picked in New Game, instead of always on its own default
- An agent that answers with nothing twice, as Pi does when the provider rejects its key, now says
  so instead of reporting that it found no legal move
- When an agent fails to move, the error shows in the error bar with the others, under the agent's
  name, instead of as a line of text in the side panel. Continue still picks the game up again

## 2.6.0 - 2026-11-08

### Added

- New Game has the kind of game down its side: Casual, with hints, take-backs and tutoring, or
  Competition, on a bullet, blitz, rapid or classical clock with no help. Scenarios are coming soon
- The clock in New Game says how fast it is: bullet, blitz, rapid or classical
- With tutoring on, an agent speaks before the first move even when you have White: ask it to
  teach you the Sicilian and it sets out the plan and marks the board before you start
- Ask an agent to teach you a line or tell you what to play and its comments name your next move
  and mark it on the board, so a lesson goes on after the first move. Otherwise they only hint
- Squares marked by an agent or in a PGN show as a ring, as on Lichess, instead of a tint that
  looked like the last move and all but vanished on dark squares

### Changed

- Cards and notices have a faint edge, a shade off their fill
- New Game keeps one height whichever kind of game is picked

### Fixed

- Changed translations reach installs that already have the language

## 2.5.0 - 2026-11-08

### Changed

- An agent's hint shows as a card above the Hint button, naming the agent and the move
- The Moves section of the game panel starts folded, leaving the opponent's words more room

### Fixed

- The instructions and the effort no longer squeeze the agent's comment in the game panel to a
  line or two

## 2.4.0 - 2026-11-08

### Added

- Two boards from Lichess: Canvas, woven cloth in slate blue and grey, and Maple, flamed maple and
  cherry

### Changed

- The reasoning effort in the game panel is a select beside its label, instead of three buttons

## 2.3.0 - 2026-11-08

### Added

- Reasoning effort can be changed during a game against an agent, from its next move on
- Agents get a time limit for each move: 3 minutes on low effort, 8 on medium and 20 on high. One
  that runs over pauses the game and says so in the error bar
- In games against an agent, Hint offers Stockfish or any agent, as Analyze does. An agent's hint
  says why

### Changed

- Release builds are named `arm64` and `x64` instead of `aarch64` and `x86_64`. Releases carry the
  old names too, so older versions still find their update
- An engine's Update button is on only when a newer release is out, which its row names
- Take back is greyed out while the opponent or a hint is thinking

### Fixed

- The screenshots in the README showed a sliver of desktop at the window's sides

## 2.2.0 - 2026-11-08

### Added

- The welcome has a second step: pick the agents you use and whether to get Stockfish (on by
  default). Finish setup installs their adapters and Stockfish before it closes, and the first
  game is against the first agent picked
- Free models come first in a model list to type in, under Free, with the rest under Premium
- Downloads and Stockfish analysis fill the loading bar as far as they've got
- Models each agent offers are kept for a day, so New Game shows them at once; older lists show
  too while they're asked for again in the background

### Changed

- Tutoring is on by default, and stays off once you turn it off for a game
- Switches slide their knob across
- The Install and Download buttons in New Game's notices start the install themselves, as well
  as opening the settings page that shows it

### Fixed

- Castling written as `0-0` or `o-o` was read as no move, so a move named in the comment could
  be played instead
- Sidebar dividers lit up and could be dragged through an open dialog

## 2.1.0 - 2026-11-08

### Added

- The Newspaper board: hatched squares, as in chess books and newspaper diagrams
- Two piece sets: Chessnut, modern and rounded, and Pixel, in 8-bit pixel art
- Two colour themes: Solarized Light, on warm paper, and Catppuccin Mocha, in dark pastels

### Changed

- The win likelihood bar takes White's and Black's colours from the theme, softer than the stark
  white and black before

### Fixed

- In light themes, the board that follows the theme had its light and dark squares swapped

## 2.0.0 - 2026-11-08

### Added

- Every agent's own models in New Game, asked from the agent (no tokens), with its default first
  and named. More than five, or none listed as for Copilot, get a list to type in
- The model playing shows next to the agent's name, the default's too
- Updates: each start looks for a newer release, and an Update button in the title bar (or
  Check for updates in the logo's menu) downloads it, under Settings, Updates. It replaces the
  program where it is, and a restart runs the new version. With no game open, the same button
  sits next to New game
- The welcome on first start shows the board and pieces picked, on a board beside the choices
- Win likelihood, and tutoring against Maia, say when they need Stockfish and download it in one
  click: in New Game, and in the game panel when likelihood is switched on during a game. Games
  waiting for it pick up once it's there. Before, the graph stayed empty without a word

### Changed

- The chosen model is set over ACP for all agents but Copilot, which still takes `--model`
- The opponent list drops "(not installed)", which the card below it already says, and marks
  OpenCode as having free options
- Opening a list in New Game closes the one already open
- Corners are a little rounder by default: 5 px instead of 4
- Notices such as "Stockfish is not installed" use the theme's own colours instead of orange,
  with their button's label in the regular text colour
- In light themes the sliders' knob is white and framed, instead of a dark grey block, and their
  frame has the same light line as the selects' instead of a dark inner edge

## 1.7.1 - 2026-11-08

### Fixed

- Downloads try again when GitHub has trouble answering, and say so when it keeps on, instead
  of a bare HTTP status
- A download gives up on a server that doesn't answer within 20 seconds, instead of waiting
  forever

## 1.7.0 - 2026-11-08

### Added

- Opponent delay setting: each opponent move takes at least this long, 1 second by default
- The error bar copies a trace: the full error, the version and platform, the game in play and
  this run's errors, with the home folder left out
- Report issue in the error bar opens a GitHub issue with the trace filled in
- Copy trace says Copied for a moment
- Agent settings at the bottom of the Review menu
- Hindi and Simplified Chinese. Picking either offers to download its Noto font, shown in the
  title bar as it runs; the welcome waits for it before letting you start
- Language plugins can name a font and where to download it
- `chessdesk reset` (`just reset`) moves everything Chessdesk keeps on the machine to the trash,
  after typing `reset` to confirm

### Changed

- Both Analyze lists show every agent, greyed out until its adapter is set up
- The new logotype, with the crown before the name
- What's missing before an opponent can play shows as a card with a button, like the other cards
- Setting descriptions sit closer together

## 1.6.0 - 2026-11-08

### Added

- A loading bar in the title bar for longer work: an agent thinking or reviewing, Stockfish
  analysis, engine downloads and adapter installs
- Logos for Copilot, OpenCode and Pi in the opponent and analysis lists
- Each agent's logo beside its name in Settings, Agents

### Changed

- Engine downloads and adapter installs carry on, and keep showing, after the settings close
- The built-in themes are called Chessdesk Dark and Chessdesk Light

### Fixed

- A pick from a settings dropdown no longer also moves the slider beneath it

## 1.5.0 - 2026-11-08

### Added

- Dropping the king on its rook, or clicking king then rook, castles
- A tutoring verdict is a card in its colour: the numbered move, the win likelihood before and
  after, and the engine's better move
- The better move is kept in the PGN as `[%better ..]`, from tutoring and analysis

## 1.4.0 - 2026-11-08

### Added

- A welcome on first start for language, theme, board, pieces and sounds, applied as picked
- Settings, Plugins shows each plugin's description in place of its folder name and licence

## 1.3.0 - 2026-11-08

### Added

- Dialogs, menus and tooltips animate in; controls fade their hover colour
- Panel sections fold and open smoothly, the room passing to the others

### Changed

- The moves log shares the room with the opponent's section in a small window
- Start game is filled; dialogs are rounder than menus at every corner radius

## 1.2.0 - 2026-11-08

### Added

- Notes for each game, kept in the PGN as the comment before the first move
- Every section of the game panel folds into its title
- The Stockfish logo by the win likelihood says where the numbers come from

### Changed

- New game is the last button after a game, and filled

## 1.1.0 - 2026-11-08

### Added

- Maia's logo in the new game dialog
- A bundled AMOLED Black theme plugin

### Changed

- Settings and Quit moved from the Game menu to the Chessdesk menu
- The Hint button stays in place, disabled, while it's not your move
- macOS: room for the window buttons, centred in the title bar

## 1.0.0 - 2026-11-07

First release.
