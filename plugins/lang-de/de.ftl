# Chessdesk auf Deutsch. Vorlage ist crates/chessdesk/assets/en.ftl.

## Menus

menu-game = Partie
menu-review = Analyse
menu-view = Ansicht
new-game = Neue Partie
open-game = Partie öffnen
save-game-as = Partie speichern unter
export-game = Partie exportieren
take-back = Zug zurücknehmen
resign = Aufgeben
settings = Einstellungen
welcome-title = Willkommen bei Chessdesk
welcome-intro = Richte es dir ein. Alles lässt sich später in den Einstellungen ändern.
welcome-start = Los geht's
quit = Beenden
analyze-with-stockfish = Mit Stockfish analysieren
review-with = Mit { $agent } besprechen
agent-settings = Agenten-Einstellungen
flip-board = Brett drehen
game-history = Partieverlauf
game-panel = Partieleiste
hide-win-likelihood = Gewinnchancen ausblenden
show-win-likelihood = Gewinnchancen einblenden
first-move = Erster Zug
last-move = Letzter Zug
previous-move = Vorheriger Zug
next-move = Nächster Zug
close-settings = Einstellungen schließen

## About

about = Über
help = Hilfe
github-repository = GitHub-Repository
check-for-updates = Nach Updates suchen…
about-description = Eine Schach-App für den Desktop, um gegen KI-Agenten und Engines zu spielen.
about-version = Version { $version }
update-to = Auf { $version } aktualisieren
restart-to-update = Zum Aktualisieren neu starten
check-now = Jetzt prüfen
checking-for-updates = Nach Updates wird gesucht
up-to-date = Version { $version }, die neueste
update-available = Version { $version } ist erschienen, installiert ist { $current }
update-ready = Version { $version } ist installiert: neu starten, um sie zu nutzen
release-notes = Versionshinweise

## Window and common buttons

minimize = Minimieren
restore = Wiederherstellen
maximize = Maximieren
close = Schließen
copy-trace = Ablauf kopieren
copied = Kopiert
report-issue = Problem melden
cancel = Abbrechen
delete = Löschen
export = Exportieren
choose = Auswählen
open = Öffnen
rename = Umbenennen
install = Installieren

## Work under way, in the title bar

activity-thinking = { $agent } denkt nach
activity-reviewing = { $agent } kommentiert die Partie
activity-analyzing = Analyse mit Stockfish, Zug { $done } von { $all }
activity-downloading = { $name } wird heruntergeladen, { $percent } %
activity-installing = Der Adapter für { $agent } wird installiert
activity-asking-models = { $agent } wird nach seinen Modellen gefragt

## Game history

move-count =
    { $count ->
        [one] 1 Zug
       *[other] { $count } Züge
    }
history-empty = Deine Partien erscheinen hier.
delete-title = { $game } löschen?
delete-hint = Sie wird in den Papierkorb verschoben.
delete-hint-running = Sie wird beendet und in den Papierkorb verschoben.
standing-won = Gewonnen
standing-lost = Verloren
draw = Remis
versus = { $white } gegen { $black }

## Game panel

no-game = Keine Partie
result-won = Gewonnen
result-lost = Verloren
ending-checkmate = durch Schachmatt
ending-stalemate = durch Patt
ending-insufficient-material = durch ungenügendes Material
ending-time = auf Zeit
ending-resignation = durch Aufgabe
verdict-perfect = Perfekt
verdict-good = Gut
verdict-inaccuracy = Ungenauigkeit
verdict-mistake = Fehler
verdict-blunder = Grober Fehler
verdict-likelihood = Gewinnchancen { $before } % → { $after } %
verdict-best = Besser war { $best }
tokens-used = { $total } Tokens, { $cached } aus dem Cache
progress = { $done } von { $all }
side-white = Weiß
side-black = Schwarz
side-random = Zufällig
win-likelihood = Gewinnchancen
likelihood-source = Von Stockfish berechnet, auf deinem Rechner
moves = Züge
notes = Notizen
notes-placeholder = Pläne, Ideen, was du dir aus dieser Partie merken willst
analyze = Analysieren
continue = Weiter
hint = Tipp
optimal-move = Bester Zug
agent-instructions = So soll der Agent spielen
instructions-example = Eröffne mit dem Londoner System. Sei nachsichtig mit mir.

## New game

opponent = Gegner
free-options = Kostenlose Modelle
install-in-settings = In den Einstellungen installieren
download-in-settings = In den Einstellungen herunterladen
strength = Spielstärke
full-strength = Volle Stärke
engine-elo = { $elo } Engine-Elo
maia-strength = Spielt etwa wie { $rating } auf Lichess
model = Modell
model-hint = Günstigere Modelle verbrauchen weniger Guthaben
model-default = Standard ({ $model })
model-agent-default = Standard des Agenten
models-asking = Modelle werden abgefragt…
models-unknown = Modelle nicht abrufbar: { $error }
reasoning-effort = Denkaufwand
effort-value = { $effort }. Mehr kostet mehr Tokens
effort-low = Niedrig
effort-medium = Mittel
effort-high = Hoch
you-play = Du spielst
time-control = Bedenkzeit
no-clock = Ohne Uhr
clock-minutes = { $minutes } Minuten
clock-increment = { $minutes } Minuten + { $increment } s
token-budget = Token-Budget
token-budget-hint = Hält die Partie an, wenn es aufgebraucht ist
no-limit = Unbegrenzt
budget-tokens = { $tokens } Tokens
tutoring = Trainingsmodus
tutoring-hint-agent = Der Agent begleitet dich während der Partie
tutoring-hint-engine = Stockfish bewertet jeden deiner Züge während der Partie
starting-position = Ausgangsstellung
position-standard = Standard
position-custom = Eigene Stellung
paste-fen = FEN einfügen
show-likelihood = Gewinnchancen während der Partie zeigen
show-likelihood-hint = Stockfish, auf deinem Rechner. Aus bleibt die Partie fair
start-game = Partie starten

## Settings

page-appearance = Darstellung
page-board = Brett
page-sound = Ton
page-engines = Engines
page-agents = Agenten
page-keys = Tastenkürzel
page-plugins = Plugins
page-updates = Updates
language = Sprache
language-system = System ({ $language })
theme = Farbschema
theme-dark = Chessdesk Dunkel
theme-light = Chessdesk Hell
pieces = Figuren
font-size = Schriftgröße
corner-radius = Eckenradius
pixels = { $value } px
board = Brett
board-follows-theme = Farbschema
board-size = Brettgröße
board-size-value = { $percent } % des verfügbaren Platzes
animate-moves = Züge animieren
animate-moves-hint = Gegnerische Züge und das Durchblättern einer Partie gleiten an ihren Platz
opponent-delay = Bedenkzeit des Gegners
opponent-delay-value = Jeder Zug des Gegners dauert mindestens { $seconds } s
sound-pack = Klangpaket
sound-off = Aus
volume = Lautstärke
volume-value = { $percent } %
downloading = Wird heruntergeladen, { $percent } %
font-needed = Diese Sprache braucht die Schrift { $font }
font-download-size = { $megabytes } MB, der Fortschritt steht in der Titelleiste
installed = Installiert
download = Herunterladen
update = Aktualisieren
choose-program = Programm auswählen
choose-folder = Ordner auswählen
not-set = Nicht festgelegt
maia-networks = Maia-Netze
installing-adapter = Der Adapter wird installiert
adapter-missing = Der Adapter ist nicht installiert
needs-node = Benötigt Node.js
program-not-found = { $program } nicht gefunden
plugins-folder = Plugin-Ordner
plugins-folder-hint = Kopiere einen Plugin-Ordner hierher, um ihn zu installieren
installed-plugins = Installierte Plugins
open-toml = TOML öffnen

## Why an opponent can't play

agent-needs-node = Dieser Agent benötigt Node.js.
agent-adapter-missing = Der Adapter für { $agent } ist nicht installiert.
stockfish-missing = Stockfish ist nicht installiert.
maia-missing = Maia benötigt lc0 und seine Netze.
stockfish-needed = { $feature ->
    [likelihood] Die Gewinnchancen brauchen Stockfish
    [tutoring] Der Trainingsmodus braucht Stockfish
   *[both] Gewinnchancen und Trainingsmodus brauchen Stockfish
}
stockfish-download-size = Etwa 80 MB, der Fortschritt steht in der Titelleiste

## Exporting

export-location = Speicherort
export-pgn = PGN mit Analyse
export-pgn-moves = PGN, nur Züge
export-fen = FEN der letzten Stellung

## Errors and notices

error-remember-choices = Die Auswahl konnte nicht gespeichert werden: { $error }
error-save-setup = Die Partieeinstellungen konnten nicht gespeichert werden: { $error }
error-save-instructions = Die Anweisungen konnten nicht gespeichert werden: { $error }
error-save-game = Die Partie konnte nicht gespeichert werden: { $error }
error-delete-game = Die Partie konnte nicht gelöscht werden: { $error }
error-export-game = Die Partie konnte nicht exportiert werden: { $error }
error-save-setting = Die Einstellung konnte nicht gespeichert werden: { $error }
error-download = Download fehlgeschlagen: { $error }
error-install = Installation fehlgeschlagen: { $error }
error-update-check = Die Suche nach Updates ist fehlgeschlagen: { $error }
error-restart = Der Neustart ist fehlgeschlagen: { $error }
error-review = Analyse: { $error }
error-no-data-folder = Auf diesem System gibt es keinen Datenordner
invalid-position = Diese Ausgangsstellung ist ungültig.
invalid-fen = Die Zwischenablage enthält keine gültige FEN.
illegal-move = Dieser Zug war hier nicht erlaubt.
budget-used = Token-Budget von { $budget } aufgebraucht.
needs-stockfish = Dafür wird Stockfish benötigt. Lade es unter Einstellungen, Engines herunter.
only-own-games-resign = Nur hier gespielte Partien können aufgegeben werden.
agent-not-set-up = { $agent } ist nicht eingerichtet. Siehe Einstellungen, Agenten.
review-empty = Die Analyse hat keine Kommentare ergänzt.
