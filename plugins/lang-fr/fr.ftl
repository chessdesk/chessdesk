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
welcome-start = C'est parti
quit = Quitter
analyze-with-stockfish = Analyser avec Stockfish
review-with = Commenter avec { $agent }
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

## Window and common buttons

minimize = Réduire
restore = Restaurer
maximize = Agrandir
close = Fermer
copy = Copier
cancel = Annuler
delete = Supprimer
export = Exporter
choose = Choisir
open = Ouvrir
rename = Renommer
install = Installer

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
hint = Indice
optimal-move = Meilleur coup
agent-instructions = Comment l'agent doit jouer
instructions-example = Ouvre avec le système de Londres. Sois indulgent avec moi.

## New game

opponent = Adversaire
not-installed = { $name } (non installé)
install-in-settings = Installer dans les réglages
download-in-settings = Télécharger dans les réglages
strength = Niveau
full-strength = Pleine puissance
engine-elo = { $elo } Elo moteur
maia-strength = Joue environ { $rating } sur Lichess
model = Modèle
model-hint = Les modèles moins chers utilisent moins de crédits
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
language = Langue
language-system = Système ({ $language })
theme = Thème
theme-dark = Sombre
theme-light = Clair
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
sound-pack = Pack de sons
sound-off = Désactivé
volume = Volume
volume-value = { $percent } %
downloading = Téléchargement, { $percent } %
installed = Installé
download = Télécharger
update = Mettre à jour
choose-program = Choisir le programme
choose-folder = Choisir le dossier
not-set = Non défini
maia-networks = Réseaux Maia
installing-adapter = Installation de l'adaptateur
adapter-missing = L'adaptateur n'est pas installé
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
