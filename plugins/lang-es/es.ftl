# Chessdesk en español. La plantilla es crates/chessdesk/assets/en.ftl.

## Menus

menu-game = Partida
menu-review = Análisis
menu-view = Ver
new-game = Nueva partida
open-game = Abrir partida
save-game-as = Guardar partida como
export-game = Exportar partida
take-back = Deshacer jugada
resign = Rendirse
settings = Ajustes
welcome-title = Te damos la bienvenida a Chessdesk
welcome-intro = Hazlo tuyo. Puedes cambiarlo todo más tarde en Ajustes.
welcome-start = A jugar
quit = Salir
analyze-with-stockfish = Analizar con Stockfish
review-with = Comentar con { $agent }
agent-settings = Ajustes de agentes
flip-board = Girar el tablero
game-history = Historial de partidas
game-panel = Panel de partida
hide-win-likelihood = Ocultar probabilidad de ganar
show-win-likelihood = Mostrar probabilidad de ganar
first-move = Primera jugada
last-move = Última jugada
previous-move = Jugada anterior
next-move = Jugada siguiente
close-settings = Cerrar ajustes

## About

about = Acerca de
help = Ayuda
github-repository = Repositorio de GitHub
check-for-updates = Buscar actualizaciones…
about-description = Una aplicación de ajedrez de escritorio para jugar contra agentes de IA y motores.
about-version = Versión { $version }

## Window and common buttons

minimize = Minimizar
restore = Restaurar
maximize = Maximizar
close = Cerrar
copy-trace = Copiar registro
copied = Copiado
report-issue = Informar del problema
cancel = Cancelar
delete = Eliminar
export = Exportar
choose = Elegir
open = Abrir
rename = Renombrar
install = Instalar

## Work under way, in the title bar

activity-thinking = { $agent } está pensando
activity-reviewing = { $agent } está comentando la partida
activity-analyzing = Analizando con Stockfish, jugada { $done } de { $all }
activity-downloading = Descargando { $name }, { $percent } %
activity-installing = Instalando el adaptador de { $agent }
activity-asking-models = Preguntando a { $agent } por sus modelos

## Game history

move-count =
    { $count ->
        [one] 1 jugada
       *[other] { $count } jugadas
    }
history-empty = Tus partidas aparecen aquí.
delete-title = ¿Eliminar { $game }?
delete-hint = Se mueve a la papelera.
delete-hint-running = Se detiene y se mueve a la papelera.
standing-won = Ganada
standing-lost = Perdida
draw = Tablas
versus = { $white } contra { $black }

## Game panel

no-game = Ninguna partida
result-won = Has ganado
result-lost = Has perdido
ending-checkmate = por jaque mate
ending-stalemate = por ahogado
ending-insufficient-material = por material insuficiente
ending-time = por tiempo
ending-resignation = por abandono
verdict-perfect = Perfecta
verdict-good = Buena
verdict-inaccuracy = Imprecisión
verdict-mistake = Error
verdict-blunder = Error grave
verdict-likelihood = Probabilidad de ganar { $before } % → { $after } %
verdict-best = Era mejor { $best }
tokens-used = { $total } tokens, { $cached } en caché
progress = { $done } de { $all }
side-white = Blancas
side-black = Negras
side-random = Al azar
win-likelihood = Probabilidad de ganar
likelihood-source = Calculado por Stockfish, en tu equipo
moves = Jugadas
notes = Notas
notes-placeholder = Planes, ideas, lo que quieras recordar de esta partida
analyze = Analizar
continue = Continuar
hint = Pista
optimal-move = Mejor jugada
agent-instructions = Cómo debe jugar el agente
instructions-example = Abre con el sistema Londres. Ten piedad de mí.

## New game

opponent = Rival
free-options = Opciones gratuitas
install-in-settings = Instalar en Ajustes
download-in-settings = Descargar en Ajustes
strength = Nivel
full-strength = Máxima fuerza
engine-elo = { $elo } Elo de motor
maia-strength = Juega en torno a { $rating } en Lichess
model = Modelo
model-hint = Los modelos más baratos usan menos créditos
model-default = Predeterminado ({ $model })
model-agent-default = El predeterminado del agente
models-asking = Consultando modelos…
models-unknown = No se pudieron consultar sus modelos: { $error }
reasoning-effort = Esfuerzo de razonamiento
effort-value = { $effort }. Cuanto más alto, más tokens
effort-low = Bajo
effort-medium = Medio
effort-high = Alto
you-play = Juegas con
time-control = Control de tiempo
no-clock = Sin reloj
clock-minutes = { $minutes } minutos
clock-increment = { $minutes } minutos + { $increment } s
token-budget = Presupuesto de tokens
token-budget-hint = Pausa la partida cuando se agota
no-limit = Sin límite
budget-tokens = { $tokens } tokens
tutoring = Modo tutor
tutoring-hint-agent = El agente te acompaña durante la partida
tutoring-hint-engine = Stockfish valora cada una de tus jugadas mientras juegas
starting-position = Posición inicial
position-standard = Estándar
position-custom = Posición personalizada
paste-fen = Pegar FEN
show-likelihood = Mostrar probabilidad de ganar al jugar
show-likelihood-hint = Stockfish, en tu equipo. Desactivado, la partida es justa
start-game = Empezar partida

## Settings

page-appearance = Apariencia
page-board = Tablero
page-sound = Sonido
page-engines = Motores
page-agents = Agentes
page-keys = Atajos
page-plugins = Complementos
language = Idioma
language-system = Sistema ({ $language })
theme = Tema
theme-dark = Chessdesk Oscuro
theme-light = Chessdesk Claro
pieces = Piezas
font-size = Tamaño de letra
corner-radius = Radio de las esquinas
pixels = { $value } px
board = Tablero
board-follows-theme = Tema
board-size = Tamaño del tablero
board-size-value = { $percent } % del espacio disponible
animate-moves = Animar jugadas
animate-moves-hint = Las jugadas del rival y el repaso de una partida se deslizan a su sitio
opponent-delay = Retardo del rival
opponent-delay-value = Cada jugada del rival tarda al menos { $seconds } s
sound-pack = Paquete de sonidos
sound-off = Desactivado
volume = Volumen
volume-value = { $percent } %
downloading = Descargando, { $percent } %
font-needed = Este idioma necesita la fuente { $font }
font-download-size = Descarga de { $megabytes } MB; el progreso aparece en la barra de título
installed = Instalado
download = Descargar
update = Actualizar
choose-program = Elegir programa
choose-folder = Elegir carpeta
not-set = Sin definir
maia-networks = Redes de Maia
installing-adapter = Instalando el adaptador
adapter-missing = El adaptador no está instalado
needs-node = Necesita Node.js
program-not-found = No se encuentra { $program }
plugins-folder = Carpeta de complementos
plugins-folder-hint = Copia aquí la carpeta de un complemento para instalarlo
installed-plugins = Complementos instalados
open-toml = Abrir TOML

## Why an opponent can't play

agent-needs-node = Este agente necesita Node.js.
agent-adapter-missing = El adaptador de { $agent } no está instalado.
stockfish-missing = Stockfish no está instalado.
maia-missing = Maia necesita lc0 y sus redes.

## Exporting

export-location = Ubicación
export-pgn = PGN con análisis
export-pgn-moves = PGN, solo jugadas
export-fen = FEN de la última posición

## Errors and notices

error-remember-choices = no se pudo recordar la selección: { $error }
error-save-setup = no se pudo guardar la configuración de la partida: { $error }
error-save-instructions = no se pudieron guardar las instrucciones: { $error }
error-save-game = no se pudo guardar la partida: { $error }
error-delete-game = no se pudo eliminar la partida: { $error }
error-export-game = no se pudo exportar la partida: { $error }
error-save-setting = no se pudo guardar el ajuste: { $error }
error-download = la descarga falló: { $error }
error-install = la instalación falló: { $error }
error-review = Análisis: { $error }
error-no-data-folder = no hay carpeta de datos en este sistema
invalid-position = Esa posición inicial no es válida.
invalid-fen = El portapapeles no contiene una FEN válida.
illegal-move = Esa jugada no era legal aquí.
budget-used = Presupuesto de { $budget } tokens agotado.
needs-stockfish = Hace falta Stockfish. Descárgalo en Ajustes, Motores.
only-own-games-resign = Solo se pueden abandonar las partidas jugadas aquí.
agent-not-set-up = { $agent } no está configurado. Consulta Ajustes, Agentes.
review-empty = El análisis no añadió comentarios.
