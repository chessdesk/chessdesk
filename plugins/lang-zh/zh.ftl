## Menus

menu-game = 对局
menu-review = 复盘
menu-view = 视图
new-game = 新对局
open-game = 打开对局
save-game-as = 对局另存为
export-game = 导出对局
take-back = 悔棋
resign = 认输
settings = 设置
welcome-title = 欢迎使用 Chessdesk
welcome-intro = 按你的喜好来设置。这些以后都可以在“设置”中更改。
welcome-start = 开始下棋
quit = 退出
analyze-with-stockfish = 用 Stockfish 分析
review-with = 用 { $agent } 复盘
agent-settings = 智能体设置
flip-board = 翻转棋盘
game-history = 对局记录
game-panel = 对局面板
hide-win-likelihood = 隐藏胜率
show-win-likelihood = 显示胜率
first-move = 第一步
last-move = 最后一步
previous-move = 上一步
next-move = 下一步
close-settings = 关闭设置

## About

about = 关于
help = 帮助
github-repository = GitHub 仓库
check-for-updates = 检查更新…
about-description = 一款可与 AI 智能体和引擎对弈的桌面国际象棋应用。
about-version = 版本 { $version }

## Window and common buttons

minimize = 最小化
restore = 还原
maximize = 最大化
close = 关闭
copy-trace = 复制错误信息
copied = 已复制
report-issue = 报告问题
cancel = 取消
delete = 删除
export = 导出
choose = 选择
open = 打开
rename = 重命名
install = 安装

## Work under way, in the title bar

activity-thinking = { $agent } 正在思考
activity-reviewing = { $agent } 正在复盘
activity-analyzing = Stockfish 正在分析，第 { $done } / { $all } 步
activity-downloading = 正在下载 { $name }，{ $percent }%
activity-installing = 正在安装 { $agent } 适配器

## Game history

move-count = { $count } 步
history-empty = 你下过的对局会显示在这里。
delete-title = 删除 { $game }？
delete-hint = 它会被移到回收站。
delete-hint-running = 它会停止并被移到回收站。
standing-won = 胜
standing-lost = 负
draw = 和棋
versus = { $white } 对 { $black }

## Game panel

no-game = 没有对局
result-won = 你赢了
result-lost = 你输了
ending-checkmate = 将杀
ending-stalemate = 逼和
ending-insufficient-material = 子力不足
ending-time = 超时
ending-resignation = 认输
verdict-perfect = 完美
verdict-good = 好棋
verdict-inaccuracy = 不精确
verdict-mistake = 错着
verdict-blunder = 败着
verdict-likelihood = 胜率 { $before }% → { $after }%
verdict-best = 最佳着法是 { $best }
tokens-used = { $total } 个 token，其中 { $cached } 个来自缓存
progress = { $done } / { $all }
side-white = 白方
side-black = 黑方
side-random = 随机
win-likelihood = 胜率
likelihood-source = 由 Stockfish 在你的电脑上计算
moves = 着法
notes = 笔记
notes-placeholder = 计划、想法、这盘棋中值得记住的地方
analyze = 分析
continue = 继续
hint = 提示
optimal-move = 最佳着法
agent-instructions = 智能体应该怎么下
instructions-example = 用伦敦体系开局。手下留情。

## New game

opponent = 对手
not-installed = { $name }（未安装）
install-in-settings = 在设置中安装
download-in-settings = 在设置中下载
strength = 棋力
full-strength = 全力
engine-elo = 引擎 Elo { $elo }
maia-strength = 在 Lichess 上约 { $rating } 分
model = 模型
model-hint = 便宜的模型消耗更少额度
reasoning-effort = 推理强度
effort-value = { $effort }。越高消耗的 token 越多
effort-low = 低
effort-medium = 中
effort-high = 高
you-play = 你执
time-control = 用时
no-clock = 不计时
clock-minutes = { $minutes } 分钟
clock-increment = { $minutes } 分钟 + { $increment } 秒
token-budget = Token 预算
token-budget-hint = 用完后暂停对局
no-limit = 不限
budget-tokens = { $tokens } 个 token
tutoring = 教学模式
tutoring-hint-agent = 智能体会在你下棋时为你讲解
tutoring-hint-engine = Stockfish 会在你下棋时评价你的每一步
starting-position = 起始局面
position-standard = 标准
position-custom = 自定义局面
paste-fen = 粘贴 FEN
show-likelihood = 对局中显示胜率
show-likelihood-hint = 由 Stockfish 在你的电脑上计算。关闭以保持公平
start-game = 开始对局

## Settings

page-appearance = 外观
page-board = 棋盘
page-sound = 声音
page-engines = 引擎
page-agents = 智能体
page-keys = 快捷键
page-plugins = 插件
language = 语言
language-system = 跟随系统（{ $language }）
theme = 主题
theme-dark = Chessdesk 深色
theme-light = Chessdesk 浅色
pieces = 棋子
font-size = 字号
corner-radius = 圆角
pixels = { $value } 像素
board = 棋盘
board-follows-theme = 跟随主题
board-size = 棋盘大小
board-size-value = 占可用空间的 { $percent }%
animate-moves = 着法动画
animate-moves-hint = 对手的着法和翻看对局时棋子会滑动到位
opponent-delay = 对手延迟
opponent-delay-value = 对手每步至少用时 { $seconds } 秒
sound-pack = 音效包
sound-off = 关闭
volume = 音量
volume-value = { $percent }%
downloading = 正在下载，{ $percent }%
font-needed = 这种语言需要 { $font } 字体
font-download-size = 下载大小 { $megabytes } MB，进度显示在标题栏
installed = 已安装
download = 下载
update = 更新
choose-program = 选择程序
choose-folder = 选择文件夹
not-set = 未设置
maia-networks = Maia 网络
installing-adapter = 正在安装适配器
adapter-missing = 适配器未安装
needs-node = 需要 Node.js
program-not-found = 找不到 { $program }
plugins-folder = 插件文件夹
plugins-folder-hint = 把插件文件夹复制到这里即可安装
installed-plugins = 已安装的插件
open-toml = 打开 TOML

## Why an opponent can't play

agent-needs-node = 这个智能体需要 Node.js。
agent-adapter-missing = { $agent } 适配器未安装。
stockfish-missing = Stockfish 未安装。
maia-missing = Maia 需要 lc0 及其网络。

## Exporting

export-location = 位置
export-pgn = 带分析的 PGN
export-pgn-moves = 仅着法的 PGN
export-fen = 最后局面的 FEN

## Errors and notices

error-remember-choices = 无法记住选择：{ $error }
error-save-setup = 无法保存对局设置：{ $error }
error-save-instructions = 无法保存指示：{ $error }
error-save-game = 无法保存对局：{ $error }
error-delete-game = 无法删除对局：{ $error }
error-export-game = 无法导出对局：{ $error }
error-save-setting = 无法保存设置：{ $error }
error-download = 下载失败：{ $error }
error-install = 安装失败：{ $error }
error-review = 复盘：{ $error }
error-no-data-folder = 系统上没有数据文件夹
invalid-position = 这个起始局面无效。
invalid-fen = 剪贴板中没有有效的 FEN。
illegal-move = 这步棋在这里不合法。
budget-used = { $budget } 个 token 的预算已用完。
needs-stockfish = 这需要 Stockfish。请在“设置 → 引擎”中下载。
only-own-games-resign = 只有在这里下的对局才能认输。
agent-not-set-up = { $agent } 尚未设置。请查看“设置 → 智能体”。
review-empty = 复盘没有添加任何评论。
