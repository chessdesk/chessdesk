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
with the `dist` profile for Windows (x86_64, ARM64), macOS (one universal `Chessdesk.app`) and
Linux (x86_64, ARM64, built on Ubuntu 22.04 for an old enough glibc).

Before anything is published, the macOS app is started on an Apple Silicon and an Intel runner, and
the Linux builds under Xvfb with a software GPU, by [smoke-test.sh](../.github/smoke-test.sh). It
fails if the app quits or panics within 20 seconds, doesn't unpack its plugins, or (on Linux) draws
nothing. Each run keeps a screenshot per system as an artifact. Look at them: a window that comes
up but renders wrong still passes.

If all of that passes, it publishes the release with all archives. So to release, bump the version.
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

Try the app without an engine, or check a real agent, which spends real tokens:

```powershell
cargo build -p chessdesk-players --example fake_uci   # then point `stockfish` at it
cargo run -p chessdesk-players --example agent_smoke -- claude haiku
```
