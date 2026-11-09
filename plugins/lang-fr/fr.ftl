# Chessdesk en français. Le modèle est crates/chessdesk/assets/en.ftl.

## Menus

menu-game = Partie
menu-review = Analyse
menu-view = Affichage
new-game = Nouvelle partie
open-game = Ouvrir une partie
save-game-as = Enregistrer la partie sous
export-game = Exporter la partie
take-back = Reprendre le coup
resign = Abandonner
settings = Réglages
welcome-title = Bienvenue dans Chessdesk
welcome-intro = Faites-en le vôtre. Tout se change plus tard dans les Réglages.
welcome-opponents = Contre qui voulez-vous jouer ?
welcome-opponents-intro = Choisissez les agents que vous utilisez, et Chessdesk installe ce qu'il leur faut. Ils jouent sur votre propre compte.
welcome-finish = Terminer la configuration
quit = Quitter
analyze-with-stockfish = Analyser avec Stockfish
review-with = Commenter avec { $agent }
agent-settings = Réglages des agents
flip-board = Retourner l'échiquier
game-history = Historique des parties
game-panel = Panneau de partie
hide-win-likelihood = Masquer les chances de gain
show-win-likelihood = Afficher les chances de gain
first-move = Premier coup
last-move = Dernier coup
previous-move = Coup précédent
next-move = Coup suivant
close-settings = Fermer les réglages

## About

about = À propos
help = Aide
github-repository = Dépôt GitHub
check-for-updates = Rechercher des mises à jour…
about-description = Une application d'échecs de bureau pour jouer contre des agents IA et des moteurs.
about-version = Version { $version }
update-to = Passer à la { $version }
restart-to-update = Redémarrer pour mettre à jour
check-now = Vérifier
checking-for-updates = Recherche de mises à jour
up-to-date = Version { $version }, la plus récente
update-available = La version { $version } est sortie, vous avez la { $current }
update-ready = La version { $version } est en place : redémarrez pour l'utiliser
release-notes = Notes de version

## Window and common buttons

minimize = Réduire
restore = Restaurer
maximize = Agrandir
close = Fermer
copy-trace = Copier la trace
copied = Copié
report-issue = Signaler le problème
cancel = Annuler
delete = Supprimer
export = Exporter
choose = Choisir
open = Ouvrir
rename = Renommer
install = Installer

## Work under way, in the title bar

activity-thinking = { $agent } réfléchit
activity-reviewing = { $agent } commente la partie
activity-analyzing = Analyse avec Stockfish, coup { $done } sur { $all }
activity-downloading = Téléchargement de { $name }, { $percent } %
activity-installing = Installation de l'adaptateur { $agent }
activity-asking-models = { $agent } est interrogé sur ses modèles

## Game history

move-count =
    { $count ->
        [one] { $count } coup
       *[other] { $count } coups
    }
history-empty = Vos parties apparaissent ici.
delete-title = Supprimer { $game } ?
delete-hint = Elle va à la corbeille.
delete-hint-running = Elle s'arrête et va à la corbeille.
standing-won = Gagnée
standing-lost = Perdue
draw = Nulle
versus = { $white } contre { $black }

## Game panel

no-game = Aucune partie
result-won = Vous avez gagné
result-lost = Vous avez perdu
ending-checkmate = par échec et mat
ending-stalemate = par pat
ending-insufficient-material = par matériel insuffisant
ending-time = au temps
ending-resignation = par abandon
verdict-perfect = Parfait
verdict-good = Bon
verdict-inaccuracy = Imprécision
verdict-mistake = Erreur
verdict-blunder = Gaffe
verdict-likelihood = Chances de gain { $before } % → { $after } %
verdict-best = Le meilleur coup : { $best }
accuracy-percent = { $percent } % de précision
you = Vous
move-marked = Stockfish a trouvé un meilleur coup ici
tokens-used = { $total } jetons, { $cached } en cache
progress = { $done } sur { $all }
side-white = Blancs
side-black = Noirs
side-random = Au hasard
win-likelihood = Chances de gain
likelihood-source = Calculé par Stockfish, sur votre machine
moves = Coups
notes = Notes
notes-placeholder = Plans, idées, ce que tu veux retenir de cette partie
analyze = Analyser
continue = Continuer
back = Retour
hint = Indice
hint-from = Indice de { $agent }
optimal-move = Meilleur coup
agent-instructions = Comment l'agent doit jouer
instructions-example = Ouvre avec le système de Londres. Sois indulgent avec moi.

## New game

opponent = Adversaire
free-options = Options gratuites
free-models = Gratuits
premium-models = Premium
install-in-settings = Installer dans les réglages
download-in-settings = Télécharger dans les réglages
strength = Niveau
full-strength = Pleine puissance
engine-elo = { $elo } Elo moteur
maia-strength = Joue environ { $rating } sur Lichess
model = Modèle
model-hint = Les modèles moins chers utilisent moins de crédits
model-default = Par défaut ({ $model })
model-agent-default = Celui de l'agent par défaut
analysis-models = Modèles pour les analyses et les indices
analysis-models-hint = Ce que chaque agent utilise quand vous appuyez sur Analyser ou lui demandez un indice.
welcome-models = Modèles pour les analyses et les indices
welcome-models-intro = Le modèle que chaque agent utilise quand vous appuyez sur Analyser ou lui demandez un indice. Vous pouvez le changer dans Réglages, Agents.
models-asking = Recherche des modèles…
models-unknown = Impossible de connaître ses modèles : { $error }
reasoning-effort = Effort de réflexion
effort-value = { $effort }. Plus il est élevé, plus il coûte de jetons
effort-low = Faible
effort-medium = Moyen
effort-high = Élevé
you-play = Vous jouez
time-control = Cadence
no-clock = Sans pendule
clock-minutes = { $minutes } minutes
clock-increment = { $minutes } minutes + { $increment } s
token-budget = Budget de jetons
token-budget-hint = Met la partie en pause une fois épuisé
no-limit = Sans limite
budget-tokens = { $tokens } jetons
tutoring = Mode entraînement
tutoring-hint-agent = L'agent vous accompagne pendant la partie
tutoring-hint-engine = Stockfish évalue chacun de vos coups pendant la partie
starting-position = Position de départ
position-standard = Standard
position-custom = Position personnalisée
mode-casual = Amicale
mode-casual-hint = Indices, reprises et tutorat
mode-competition = Compétition
mode-competition-hint = À la pendule, sans aide
mode-scenario = Scénario
mode-scenario-hint = Partir d'une position donnée
coming-soon = Bientôt disponible
speed-bullet = Bullet
speed-blitz = Blitz
speed-rapid = Rapide
speed-classical = Classique
scenario = Position
scenario-endgame = Finale
scenario-opening = Ouverture
scenario-queen-mate = Mat avec roi et dame
scenario-rook-mate = Mat avec roi et tour
scenario-king-pawn = Roi et pion
scenario-lucena = Position de Lucena
scenario-philidor = Position de Philidor
scenario-italian = Partie italienne
scenario-sicilian = Défense sicilienne
scenario-queens-gambit = Gambit dame
scenario-french = Défense française
scenario-own = Votre propre position
scenario-own-hint = Un FEN du presse-papiers
paste-fen = Coller une FEN
show-likelihood = Afficher les chances de gain en jouant
show-likelihood-hint = Stockfish, sur votre machine. Désactivé, la partie reste équitable
start-game = Commencer la partie

## Settings

page-appearance = Apparence
page-board = Échiquier
page-sound = Son
page-engines = Moteurs
page-agents = Agents
page-keys = Raccourcis
page-plugins = Extensions
page-updates = Mises à jour
language = Langue
language-system = Système ({ $language })
theme = Thème
theme-dark = Chessdesk Sombre
theme-light = Chessdesk Clair
pieces = Pièces
font-size = Taille du texte
corner-radius = Arrondi des angles
pixels = { $value } px
board = Échiquier
board-follows-theme = Thème
board-size = Taille de l'échiquier
board-size-value = { $percent } % de la place disponible
animate-moves = Animer les coups
animate-moves-hint = Les coups adverses et le parcours d'une partie glissent en place
opponent-delay = Délai de l'adversaire
opponent-delay-value = Chaque coup de l'adversaire prend au moins { $seconds } s
sound-pack = Pack de sons
sound-off = Désactivé
volume = Volume
volume-value = { $percent } %
downloading = Téléchargement, { $percent } %
font-needed = Cette langue a besoin de la police { $font }
font-download-size = Téléchargement de { $megabytes } Mo, suivi dans la barre de titre
installed = Installé
engine-latest = Installé, la dernière ({ $version })
engine-newer = { $version } est sortie
download = Télécharger
update = Mettre à jour
choose-program = Choisir le programme
choose-folder = Choisir le dossier
not-set = Non défini
maia-networks = Réseaux Maia
installing-adapter = Installation de l'adaptateur
adapter-missing = L'adaptateur n'est pas installé
adapter-to-install = Chessdesk installe son adaptateur
stockfish-offer = Toujours inclus : comme adversaire, pour les indices et pour analyser vos parties. Environ 80 Mo
needs-node = Nécessite Node.js
program-not-found = { $program } introuvable
plugins-folder = Dossier des extensions
plugins-folder-hint = Copiez ici le dossier d'une extension pour l'installer
installed-plugins = Extensions installées
open-toml = Ouvrir le TOML

## Why an opponent can't play

agent-needs-node = Cet agent nécessite Node.js.
agent-adapter-missing = L'adaptateur de { $agent } n'est pas installé.
stockfish-missing = Stockfish n'est pas installé.
maia-missing = Maia nécessite lc0 et ses réseaux.
stockfish-needed = { $feature ->
    [likelihood] Les chances de gain nécessitent Stockfish
    [tutoring] Le mode entraînement nécessite Stockfish
   *[both] Les chances de gain et le mode entraînement nécessitent Stockfish
}
stockfish-download-size = Environ 80 Mo, suivi dans la barre de titre

## Exporting

export-location = Emplacement
export-pgn = PGN avec analyse
export-pgn-moves = PGN, coups seuls
export-fen = FEN de la dernière position

## Errors and notices

error-remember-choices = impossible de mémoriser les choix : { $error }
error-save-setup = impossible d'enregistrer les paramètres de la partie : { $error }
error-save-instructions = impossible d'enregistrer les consignes : { $error }
error-save-game = impossible d'enregistrer la partie : { $error }
error-delete-game = impossible de supprimer la partie : { $error }
error-export-game = impossible d'exporter la partie : { $error }
error-save-setting = impossible d'enregistrer le réglage : { $error }
error-download = échec du téléchargement : { $error }
error-install = échec de l'installation : { $error }
error-timed-out = { $agent } n'a pas joué en { $minutes } minutes (effort de réflexion { $effort }). Continuer redemande ; un effort moindre répond plus vite.
error-update-check = Impossible de rechercher des mises à jour : { $error }
error-restart = Impossible de redémarrer : { $error }
error-review = Analyse : { $error }
error-no-data-folder = aucun dossier de données sur ce système
invalid-position = Cette position de départ n'est pas valide.
invalid-fen = Le presse-papiers ne contient pas de FEN valide.
illegal-move = Ce coup n'était pas légal ici.
budget-used = Budget de { $budget } jetons épuisé.
needs-stockfish = Il faut Stockfish. Téléchargez-le dans Réglages, Moteurs.
only-own-games-resign = Seules les parties jouées ici peuvent être abandonnées.
agent-not-set-up = { $agent } n'est pas configuré. Voir Réglages, Agents.
review-empty = L'analyse n'a ajouté aucun commentaire.
