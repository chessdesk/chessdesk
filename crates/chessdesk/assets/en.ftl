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
welcome-opponents = Who do you want to play?
welcome-models = Models for reviews and hints
welcome-models-intro = The model each agent uses when you press Analyze or ask it for a hint. You can change them under Settings, Agents.
welcome-opponents-intro = Pick the agents you use, and Chessdesk sets up what they need. They play on your own account.
welcome-finish = Finish setup
quit = Quit
analyze-with-stockfish = Analyze with Stockfish
review-with = Review with { $agent }
agent-settings = Agent settings
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
update-to = Update to { $version }
restart-to-update = Restart to update
check-now = Check now
checking-for-updates = Checking for updates
up-to-date = Version { $version }, the latest
update-available = Version { $version } is out, you have { $current }
update-ready = Version { $version } is in place: restart to use it
release-notes = Release notes

## Window and common buttons

minimize = Minimize
restore = Restore
maximize = Maximize
close = Close
copy-trace = Copy trace
copied = Copied
report-issue = Report issue
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
activity-downloading = Downloading { $name }, { $percent }%
activity-installing = Installing the { $agent } adapter
activity-asking-models = Asking { $agent } for its models

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
accuracy-percent = { $percent }% accuracy
you = You
move-marked = Stockfish found a better move here
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
back = Back
hint = Hint
hint-from = Hint from { $agent }
optimal-move = Optimal move
agent-instructions = How the agent should play
instructions-example = Open with the London System. Go easy on me.

## New game

opponent = Opponent
free-options = Free options
free-models = Free
premium-models = Premium
install-in-settings = Install in Settings
download-in-settings = Download in Settings
strength = Strength
full-strength = Full strength
engine-elo = { $elo } engine Elo
maia-strength = Plays around { $rating } on Lichess
model = Model
model-hint = Cheaper models use fewer credits
model-default = Default ({ $model })
model-agent-default = The agent's default
analysis-models = Models for reviews and hints
analysis-models-hint = What each agent uses when you press Analyze or ask it for a hint.
models-asking = Asking for models…
models-unknown = Could not ask for its models: { $error }
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
mode-casual = Casual
mode-casual-hint = Hints, take-backs and tutoring
mode-competition = Competition
mode-competition-hint = On the clock, on your own
mode-scenario = Scenario
mode-scenario-hint = Start from a set position
coming-soon = Coming soon
speed-bullet = Bullet
speed-blitz = Blitz
speed-rapid = Rapid
speed-classical = Classical
scenario = Position
scenario-endgame = Endgame
scenario-opening = Opening
scenario-queen-mate = Mate with king and queen
scenario-rook-mate = Mate with king and rook
scenario-king-pawn = King and pawn
scenario-lucena = Lucena position
scenario-philidor = Philidor position
scenario-italian = Italian Game
scenario-sicilian = Sicilian Defence
scenario-queens-gambit = Queen's Gambit
scenario-french = French Defence
scenario-own = Your own position
scenario-own-hint = A FEN from the clipboard
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
page-updates = Updates
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
opponent-delay = Opponent delay
opponent-delay-value = Each opponent move takes at least { $seconds } s
sound-pack = Sound pack
sound-off = Off
volume = Volume
volume-value = { $percent }%
downloading = Downloading, { $percent }%
font-needed = This language needs the { $font } font
font-download-size = A { $megabytes } MB download, shown in the title bar as it runs
installed = Installed
engine-latest = Installed, the latest ({ $version })
engine-newer = { $version } is out
download = Download
update = Update
choose-program = Choose program
choose-folder = Choose folder
not-set = Not set
maia-networks = Maia networks
installing-adapter = Installing the adapter
adapter-missing = The adapter isn't installed
adapter-to-install = Chessdesk installs its adapter
stockfish-offer = Always included: to play against, for hints and to analyse your games. About 80 MB
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
stockfish-needed = { $feature ->
    [likelihood] Win likelihood needs Stockfish
    [tutoring] Tutoring needs Stockfish
   *[both] Win likelihood and tutoring need Stockfish
}
stockfish-download-size = About 80 MB, shown in the title bar as it downloads

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
error-timed-out = { $agent } gave no move within { $minutes } minutes (reasoning effort { $effort }). Continue asks again; a lower effort answers sooner.
error-update-check = could not check for updates: { $error }
error-restart = could not restart: { $error }
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
