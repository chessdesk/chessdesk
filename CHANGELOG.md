# Changelog

Notable changes per version. The format follows [Keep a Changelog](https://keepachangelog.com):
minor versions bring features, patch versions only fixes.

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
