# Chessdesk's interface, and the base of every translation: a language plugin may leave
# out any message, which then shows in English. The format is Project Fluent:
# https://projectfluent.org/fluent/guide/

## Menus

menu-game = Game
menu-review = Review
menu-view = View
new-game = New game
open-game = Open game
save-game-as = Save game as
export-game = Export game
take-back = Take back
resign = Resign
settings = Settings
welcome-title = Welcome to Chessdesk
welcome-intro = Make it yours. You can change all of this later under Settings.
welcome-start = Let's play
quit = Quit
analyze-with-stockfish = Analyze with Stockfish
review-with = Review with { $agent }
flip-board = Flip board
game-history = Game history
game-panel = Game panel
hide-win-likelihood = Hide win likelihood
show-win-likelihood = Show win likelihood
first-move = First move
last-move = Last move
previous-move = Previous move
next-move = Next move
close-settings = Close settings

## About

about = About
help = Help
github-repository = GitHub Repository
check-for-updates = Check for updates…
about-description = A desktop chess app for playing AI agents and engines.
about-version = Version { $version }

## Window and common buttons

minimize = Minimize
restore = Restore
maximize = Maximize
close = Close
copy = Copy
cancel = Cancel
delete = Delete
export = Export
choose = Choose
open = Open
rename = Rename
install = Install

## Work under way, in the title bar

activity-thinking = { $agent } is thinking
activity-reviewing = { $agent } is reviewing the game
activity-analyzing = Analyzing with Stockfish, move { $done } of { $all }
activity-downloading = Downloading { $engine }, { $percent }%
activity-installing = Installing the { $agent } adapter

## Game history

move-count =
    { $count ->
        [one] 1 move
       *[other] { $count } moves
    }
history-empty = Games you play show up here.
delete-title = Delete { $game }?
delete-hint = It moves to the Recycle Bin.
delete-hint-running = It stops and moves to the Recycle Bin.
standing-won = Won
standing-lost = Lost
draw = Draw
versus = { $white } vs { $black }

## Game panel

no-game = No game
result-won = You won
result-lost = You lost
ending-checkmate = by checkmate
ending-stalemate = by stalemate
ending-insufficient-material = by insufficient material
ending-time = on time
ending-resignation = by resignation
verdict-perfect = Perfect
verdict-good = Good
verdict-inaccuracy = Inaccuracy
verdict-mistake = Mistake
verdict-blunder = Blunder
verdict-likelihood = Win likelihood { $before }% → { $after }%
verdict-best = Best was { $best }
tokens-used = { $total } tokens, { $cached } cached
progress = { $done } of { $all }
side-white = White
side-black = Black
side-random = Random
win-likelihood = Win likelihood
likelihood-source = Worked out by Stockfish, on your machine
moves = Moves
notes = Notes
notes-placeholder = Plans, ideas, what to remember from this game
analyze = Analyze
continue = Continue
hint = Hint
optimal-move = Optimal move
agent-instructions = How the agent should play
instructions-example = Open with the London System. Go easy on me.

## New game

opponent = Opponent
not-installed = { $name } (not installed)
install-in-settings = Install in Settings
download-in-settings = Download in Settings
strength = Strength
full-strength = Full strength
engine-elo = { $elo } engine Elo
maia-strength = Plays around { $rating } on Lichess
model = Model
model-hint = Cheaper models use fewer credits
reasoning-effort = Reasoning effort
effort-value = { $effort }. Higher costs more tokens
effort-low = Low
effort-medium = Medium
effort-high = High
you-play = You play
time-control = Time control
no-clock = No clock
clock-minutes = { $minutes } minutes
clock-increment = { $minutes } minutes + { $increment } s
token-budget = Token budget
token-budget-hint = Pauses the game when used up
no-limit = No limit
budget-tokens = { $tokens } tokens
tutoring = Tutoring mode
tutoring-hint-agent = The agent talks you through the game as you play
tutoring-hint-engine = Stockfish rates each of your moves as you play
starting-position = Starting position
position-standard = Standard
position-custom = Custom position
paste-fen = Paste FEN
show-likelihood = Show win likelihood during play
show-likelihood-hint = Stockfish, on your machine. Off keeps it a fair game
start-game = Start game

## Settings

page-appearance = Appearance
page-board = Board
page-sound = Sound
page-engines = Engines
page-agents = Agents
page-keys = Keys
page-plugins = Plugins
language = Language
language-system = System ({ $language })
theme = Theme
theme-dark = Chessdesk Dark
theme-light = Chessdesk Light
pieces = Pieces
font-size = Font size
corner-radius = Corner radius
pixels = { $value } px
board = Board
board-follows-theme = Theme
board-size = Board size
board-size-value = { $percent }% of the room there is
animate-moves = Animate moves
animate-moves-hint = Opponent moves and stepping through a game glide into place
sound-pack = Sound pack
sound-off = Off
volume = Volume
volume-value = { $percent }%
downloading = Downloading, { $percent }%
installed = Installed
download = Download
update = Update
choose-program = Choose program
choose-folder = Choose folder
not-set = Not set
maia-networks = Maia networks
installing-adapter = Installing the adapter
adapter-missing = The adapter isn't installed
needs-node = Needs Node.js
program-not-found = { $program } not found
plugins-folder = Plugins folder
plugins-folder-hint = Copy a plugin folder here to install it
installed-plugins = Installed plugins
open-toml = Open TOML

## Why an opponent can't play

agent-needs-node = This agent needs Node.js.
agent-adapter-missing = The { $agent } adapter isn't installed.
stockfish-missing = Stockfish is not installed.
maia-missing = Maia needs lc0 and its networks.

## Exporting

export-location = Location
export-pgn = PGN with analysis
export-pgn-moves = PGN, moves only
export-fen = FEN of the last position

## Errors and notices

error-remember-choices = could not remember the choices: { $error }
error-save-setup = could not save the game's setup: { $error }
error-save-instructions = could not save the instructions: { $error }
error-save-game = could not save the game: { $error }
error-delete-game = could not delete the game: { $error }
error-export-game = could not export the game: { $error }
error-save-setting = could not save the setting: { $error }
error-download = download failed: { $error }
error-install = install failed: { $error }
error-review = Review: { $error }
error-no-data-folder = no data folder on this system
invalid-position = That starting position is not valid.
invalid-fen = The clipboard holds no valid FEN.
illegal-move = That move was not legal here.
budget-used = Token budget of { $budget } used up.
needs-stockfish = This needs Stockfish. Download it under Settings, Engines.
only-own-games-resign = Only games played here can be resigned.
agent-not-set-up = { $agent } isn't set up. See Settings, Agents.
review-empty = The review added no comments.
