# Development

## Build

Needs [rustup](https://rustup.rs) and [just](https://just.systems). On Windows, also Visual Studio
Build Tools with the C++ tools and the Windows SDK. On ARM64, GPUI's dependency `ring` needs the
official LLVM clang (`winget install LLVM.LLVM`) set as `CC_aarch64_pc_windows_msvc`.

```powershell
just run       # start Chessdesk
just release   # optimized binary at target\release\chessdesk.exe
just dist      # fully optimized (fat LTO), as released, at target\dist\chessdesk.exe
just check     # fmt, lint, test, deny
just hooks     # git hooks, once per clone; needs prek
just reset     # start over: config, plugins, adapters, engines, fonts and games to the trash,
               # once you type "reset" (also `cargo run -- reset`)
```

The binary carries its font, themes and plugins, so it needs no installing.

## Release

CI runs fmt, clippy and the tests on Linux and macOS. Windows is covered by the pre-push hook
(`just hooks`), so push from Windows with the hooks installed.

Every push to `main` runs [release.yml](../.github/workflows/release.yml) once CI passes, but it only
builds when the workspace version in `Cargo.toml` has no `v<version>` release yet. Then it builds
with the `dist` profile for Windows (x64, ARM64), macOS (one universal `Chessdesk.app`) and
Linux (x64, ARM64, built on Ubuntu 22.04 for an old enough glibc).

Before anything is published, the macOS app is started on an Apple Silicon and an Intel runner, and
the Linux builds under Xvfb with a software GPU, by [smoke-test.sh](../.github/smoke-test.sh). It
fails if the app quits or panics within 20 seconds, doesn't unpack its plugins, or (on Linux) draws
nothing. Each run keeps a screenshot per system as an artifact. Look at them: a window that comes
up but renders wrong still passes.

If all of that passes, it publishes the release with all archives. So to release, bump the version.
The app finds it through GitHub's latest release and downloads the archive named for its system
([update.rs](../crates/chessdesk-players/src/update.rs)), so keep the archive names as they are.
Only optimized builds look at launch: `just release` to try the update button against the last
release with an older version in `Cargo.toml`.
To try the builds from a branch without publishing: Actions, Release, Run workflow.

## Configure

`Ctrl+,` opens the settings page. It writes to `config.toml` in Chessdesk's config folder
(`%APPDATA%\chessdesk` on Windows, or `$CHESSDESK_CONFIG_DIR`), which can also be edited by hand:
saving the file applies it. All keys and defaults are in
[default_config.toml](../crates/chessdesk/assets/default_config.toml).

```toml
theme = "Mariana"
piece_set = "Merida"
board_size = 0.7        # of the room there is, from 0.5 to 1
animate_moves = false
stockfish = 'C:\tools\stockfish.exe'

[colors]                # override single colours of the theme
board_dark = "#2f3b3b"
```

## Layout

| Path | Holds |
|---|---|
| `crates/chessdesk-core` | The game: rules, PGN, annotations, review logic, plugin manifests. No UI, no processes. |
| `crates/chessdesk-players` | Opponents and analysts: UCI client, ACP sessions, prompts, reading replies. |
| `crates/chessdesk` | The GPUI app. |
| `plugins` | The plugins bundled into the binary. |

The solid piece icons are [Font Awesome Free](https://fontawesome.com) (CC BY 4.0). The Claude Code, Codex, Copilot and OpenCode logos come from [Lobe Icons](https://github.com/lobehub/lobe-icons) (MIT) and remain their owners' marks. The Pi logo comes from [pi.dev](https://pi.dev) and remains its owners' mark. The Stockfish logo in `assets` comes from [stockfish-web](https://github.com/official-stockfish/stockfish-web) (MIT). The Maia logo is the favicon of [maiachess.com](https://www.maiachess.com) and remains its owners' mark.

## Scripts

`chessdesk script <file>` drives the app the way a player would and takes screenshots, for
checking a change by eye without clicking through it. It uses your own config, engines and agents;
the games it plays land in the history unless the script ends with `discard`.

```powershell
just script crates/chessdesk/scripts/mate.txt       # same every run, no tokens
just script crates/chessdesk/scripts/analysis.txt   # a lost game, its analysis and a Pi review
```

Screenshots go next to the script (`crates/chessdesk/scripts/shots/`, ignored by git). The log
goes to stderr, one line per command, and the process exits 0 when the script went through, 1 when
a command failed or the app reported an error, and 2 when the script couldn't be read. Use a
debug build: a release build on Windows has no console to print to.

One command per line; `#` starts a comment, and paths are relative to the script. Each command
first waits until the game on show has settled: no opponent or agent at work, no evaluation or
analysis running, no pieces moving, and no agent being asked for its models or efforts.

| Command | Does |
|---|---|
| `new white\|black [elo N] [shadowchess LEVEL] [fen FEN]` | A casual game against Stockfish, at 1320 unless `elo` says otherwise, or against Shadowchess at `LEVEL` (500, 1000 or 1500), from the start or the FEN (the rest of the line). New Game's remembered choices stay as they were. |
| `open PATH` | Opens a saved game. |
| `play SAN...` | Plays the player's moves, each once the opponent has answered the one before. |
| `resign` | Resigns the game on show. |
| `analyze` | Runs Stockfish over the game, as the Analyze button does. |
| `review AGENT [model MODEL]` | An agent's review: `claude-code`, `codex`, `copilot`, `opencode` or `pi`, on a model as the agent names it (New Game's model list shows them), e.g. `review pi model openrouter/z-ai/glm-5.3-flash`. Spends tokens. |
| `show N\|end` | Shows the position after N half-moves, or the last. |
| `screenshot PATH` | Saves the window as a PNG, after waiting for animations to finish. |
| `settings PAGE` | Opens a settings page in the board's place: `appearance`, `board`, `sound`, `engines`, `agents`, `keys`, `plugins` or `updates`. |
| `timeout SECONDS` | How long later commands may wait for the game to settle (default 60). Raise it for agents. |
| `discard` | Moves the game on show to the Recycle Bin. |

Screenshots: on Windows, the window paints itself into the picture (`PrintWindow`), so it can be
covered or on another desktop. macOS uses `screencapture` on the window's part of the screen, and
Linux ImageMagick's `import` under X11 only. Neither has been tried yet, and both need the window
uncovered.

To add a command: a variant in `script.rs` and its parsing, then what it does in
`app/scripted.rs`. If it starts work that takes a while, make `Chessdesk::is_settled` wait for it,
or the next command runs too early.

## Examples

Try the app without an engine, or check a real agent, which spends real tokens:

```powershell
cargo build -p chessdesk-players --example fake_uci   # then point `stockfish` at it
cargo run -p chessdesk-players --example agent_smoke -- claude haiku
cargo run -p chessdesk-players --example agent_models   # the models each agent offers, no tokens
```

A whole tutoring game, headless: Stockfish at a set engine Elo plays White for you, its moves graded
as the app grades yours, and the agent coaches as Black. It prints each move with its comment and
tokens, and saves `agent-game.pgn` in the temp folder. A game costs about 150,000 tokens with Sonnet,
mostly cached:

```powershell
$env:STOCKFISH = "$env:LOCALAPPDATA\chessdesk\engines\stockfish\stockfish\stockfish-windows-arm64-universal.exe"
cargo run -p chessdesk-players --example agent_game -- claude sonnet low 1320 "I'm a new player"
cargo run -p chessdesk-players --example agent_game -- copilot "" low 1320 "I'm a new player"
```

The lesson test is the sharpest check of the harness: a new player asks "teach me a super fun,
fast way to mate" and plays exactly the move the agent marks with `[%next]`. Each run is judged on
five checks (the intro names the first move, every move is named, the player gives mate, comments
read whole, Stockfish isn't named), and the last argument is the number of runs. It exits with 1 on
any failure, and costs a few thousand tokens a run:

```powershell
cargo run -p chessdesk-players --example agent_lesson -- claude opus low 3
cargo run -p chessdesk-players --example agent_lesson -- pi openrouter/z-ai/glm-5.3-flash low 3
```

Results on 2026-11-08: Opus 5.5 on low passed every check in 3 of 3 runs, mating in 4 moves for
about 8,500 tokens; GLM 5.3 Flash on low passed 2 of 3, failing when it named the move in words
only.
