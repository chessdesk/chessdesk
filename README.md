<p align="center"><img src="docs/screenshot.jpg" alt="Chessdesk: a game against Claude Code, with the tutor pointing out a hanging knight"></p>

**Chessdesk** is a chess app for your desktop. You can play Stockfish and Maia, and you can also play
the AI agents you already use, like Claude Code, Codex or Copilot. They'll play you, and if you want,
they'll talk you through the game: what they think of your last move, what they're planning, and
where you're about to lose a piece, with arrows on the board.

It's free, open source and runs on your own machine. Your games stay there as plain PGN files.

## What you can do

**Play whoever you like.** Pick an opponent when you start a game. Stockfish goes from about 1320
engine Elo up to full strength. Maia is trained on human games, so it makes human mistakes. With an
agent you choose how hard it thinks, and you can tell it how to play in your own words: "Open with
the London System", "Go easy on me", "Play like it's 1852".

**Learn while you play.** Turn on tutoring mode and an agent comments as you go, or Stockfish rates
each of your moves against an engine. Hint shows you a good move when you're stuck, and it costs no
tokens.

**Go back over your games.** Every game is saved as you play. Open one from the sidebar and press
Analyze: Stockfish checks each move and marks your mistakes, or an agent goes through your moves one
by one and explains them. The bar and graph show how the game swung. Notes you write next to a game
are saved in its PGN file, and agents never see them.

**Keep several games going.** Start another game and the first one keeps running, clocks included.
A dot marks the unfinished ones, and you can drop back in even after a restart.

**Make it yours.** There are piece sets, boards, colour themes and sounds to choose from, and it
speaks English, German, French and Spanish. It picks up your system's language by itself, and the
first start asks for the main choices. Adding your own set or language means copying a folder, see
[docs/plugins.md](docs/plugins.md).

## Getting it

Download it from [Releases](https://github.com/chessdesk/chessdesk/releases): a zip for Windows, a
universal app for macOS and a tarball for Linux. Each is a single program with nothing to install. Or
[build it yourself](docs/development.md).

It's been used on Windows so far. The macOS and Linux builds are started on those systems before every
release, but nobody has played on them yet, so reports are welcome.

Chessdesk looks for a new release at each start. When there is one, an Update button shows in the title
bar: it downloads the new version and puts it in place of the old one, wherever that is, and the new
version runs once you restart. The folder holding Chessdesk has to be writable for that. Settings,
Updates checks by hand.

The builds aren't signed, so your system asks before the first start:

- **Windows**: SmartScreen says it protected your PC. Click More info, then Run anyway.
- **macOS**: open Chessdesk once, then System Settings, Privacy & Security, Open Anyway. Or run
  `xattr -dr com.apple.quarantine Chessdesk.app`.
- **Linux**: needs a Vulkan driver (`mesa-vulkan-drivers` on Debian and Ubuntu).

## Setting up opponents

Everything is under Settings (`Ctrl+,`), and the new game dialog tells you if something's missing.

| Opponent | What you need |
|---|---|
| Stockfish | Nothing, download it under Settings, Engines |
| Maia | The same on Windows. Elsewhere, install [lc0](https://lczero.org) yourself |
| Claude Code, Codex | [Node.js](https://nodejs.org) and your login, then install the adapter under Settings, Agents |
| Copilot | The [Copilot CLI](https://github.com/github/copilot-cli) and your login |
| OpenCode | [OpenCode](https://opencode.ai), set up with a provider |
| Pi | [Pi](https://github.com/earendil-works/pi), set up with a provider, Node.js, and the adapter |

## What agents cost

Agents use your own account, so moves cost tokens. Chessdesk tries hard to keep that small: the rules,
the clock and the analysis all run on your machine, and an agent only gets the last move and the
position. A move is about 1,000 tokens on Claude, most of it cached, and reviewing a short game about
2,000. You can set a token budget for a game, which pauses it when it's used up. Hover over the
opponent's name to see what a game has used so far.

If an agent can't come up with a legal move, it gets one more try with the legal moves spelled out.
After that the game pauses and you decide. Chessdesk never makes up a move for it.

## Keys

| Key | What it does |
|---|---|
| `Ctrl+N` | New game |
| `Ctrl+O`, `Ctrl+S` | Open or save a PGN |
| `Ctrl+E` | Export a game |
| `Ctrl+F` | Flip the board |
| `Ctrl+Z` | Take back |
| `Left`, `Right`, `Home`, `End` | Step through the moves |
| `Ctrl+B`, `Ctrl+Shift+B` | Show or hide the game history and the game panel |
| `Ctrl+,` | Settings |

## Licence

Copyright © 2026 Moritz Maibaum. Chessdesk is free software under the GNU General Public License,
version 3 or later: see [`LICENSE`](LICENSE). The plugins have their own licences, listed in
[docs/plugins.md](docs/plugins.md). How agents are kept cheap is measured in
[docs/design.md](docs/design.md).
