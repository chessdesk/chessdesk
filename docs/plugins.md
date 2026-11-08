# Plugins

A plugin is a folder in `<config folder>/plugins` with a `plugin.toml`. Copying one there installs
it, while Chessdesk runs; deleting it removes it, and the ones Chessdesk comes with return on the next
start. Plugins are data only. A broken one is named in the error ribbon and the others still load.

```toml
name = "walnut"
version = "1.0.0"
description = "Wooden pieces"    # optional; shown under Settings, Plugins
license = "CC0-1.0"              # of the artwork

[[pieces]]                       # any number
name = "Walnut"                  # for `piece_set = "..."`
dir = "svg"                      # holds wK.svg, wQ.svg ... bN.svg, bP.svg, all 12

[[themes]]                       # any number
name = "Walnut"                  # for `theme = "..."`
file = "walnut.toml"

[[boards]]                       # any number
name = "Walnut"                  # for `board = "..."`
image = "board.jpg"              # all 64 squares, a8 top left, PNG or JPEG
light = "#e3c594"                # for the coordinates on the squares
dark = "#8a5a34"

[[sounds]]                       # any number
name = "Walnut"                  # for `sound_pack = "..."`
dir = "sounds"                   # holds move.ogg and any of the others below, OGG or WAV

[[languages]]                    # any number
name = "Frysk"                   # in the language itself, for `language = "..."`
locale = "fy"                    # BCP 47: picks its plural rules, and matches the system's
file = "fy.ftl"                  # Project Fluent

[languages.font]                 # optional: a font for the language's script
family = "Noto Sans SC"          # as the font file names it
url = "https://…/NotoSansSC-Regular.otf"  # https, .ttf or .otf; best a fixed version
bytes = 8331336                  # optional: the size, shown before downloading
```

A board looks the same under any colour theme. `board = "Theme"` follows the theme instead.

A sound pack needs `move`. The others are `capture`, `check` and `castle`, which sound like a move
if left out, and `game-start`, `win`, `lose`, `draw` and `low-time`, which stay silent. Keep them
short, and at most 4 MiB each. `sound_pack = "Off"` turns sound off.

A language needs no more than the messages it translates: the rest show in English, so a translation
can start with a few lines and grow. Copy [`en.ftl`](../crates/chessdesk/assets/en.ftl), which lists every
message, and keep the `{ $names }` in each. Plurals go by the language's own categories:

```ftl
new-game = Nij spul
move-count =
    { $count ->
        [one] 1 set
       *[other] { $count } setten
    }
```

Save the file and Chessdesk reloads it. `language = "System"`, the default, selects the system's
language where a plugin has it and English otherwise; the setting shows which it came to. Agents write their comments in the chosen language too; moves
stay in standard English notation, as do messages from engines and the plugin errors in the ribbon.
Scripts the interface font lacks come from the system's fonts, or from the font a language names.
Picking the language offers to download that font, and once it's there it replaces the interface
font, so it must cover Latin as well; the Noto fonts from Google Fonts do. Downloaded fonts are kept
in the `fonts` folder next to the engines.

A theme file needs `background`, `accent`, `popover`, `popover_border`, `text`, `muted`, `caret`,
`callout` and 16 `ansi` colours, of which red, green, yellow and blue draw markings. `board_light`
and `board_dark` are optional; other keys are ignored. The
[built-in dark theme](../crates/chessdesk/assets/themes/dark.toml) is a complete example.

| Plugin | Brings | Licence |
|---|---|---|
| [`cburnett`](../plugins/cburnett) | Pieces by Colin M.L. Burnett | GPL 2.0 or later |
| [`merida`](../plugins/merida) | Pieces by Armando Hernandez Marroquin | GPL 2.0 or later |
| [`chessnut`](../plugins/chessnut) | Pieces by Alexis Luengas | Apache 2.0 |
| [`pixel`](../plugins/pixel) | Pixel-art pieces by therealqtpi | AGPL 3.0 or later |
| [`mariana`](../plugins/mariana), [`monokai`](../plugins/monokai), [`celeste`](../plugins/celeste), [`amoled`](../plugins/amoled) | Colour themes | GPL 3.0 or later |
| [`solarized`](../plugins/solarized), [`catppuccin`](../plugins/catppuccin) | Solarized Light and Catppuccin Mocha, after their palettes (MIT) | GPL 3.0 or later |
| [`wood-board`](../plugins/wood-board) | Lichess's wood4 board | AGPL 3.0 or later |
| [`green-board`](../plugins/green-board) | A green board, its texture made for Chessdesk | GPL 3.0 or later |
| [`marble-board`](../plugins/marble-board) | Lichess's blue-marble board | AGPL 3.0 or later |
| [`newspaper-board`](../plugins/newspaper-board) | Lichess's newspaper board, hatched like a printed diagram | AGPL 3.0 or later |
| [`wood-sounds`](../plugins/wood-sounds) | Real wood knocks from [Kenney](https://kenney.nl/assets/impact-sounds) (CC0), and a marimba made for Chessdesk | GPL 3.0 or later |
| [`soft-sounds`](../plugins/soft-sounds) | Soft taps and a marimba, made for Chessdesk | GPL 3.0 or later |
| [`lang-de`](../plugins/lang-de), [`lang-es`](../plugins/lang-es), [`lang-fr`](../plugins/lang-fr), [`lang-hi`](../plugins/lang-hi), [`lang-zh`](../plugins/lang-zh) | German, Spanish, French, Hindi and Simplified Chinese; Hindi and Chinese download a Noto font (OFL) when picked | GPL 3.0 or later |
