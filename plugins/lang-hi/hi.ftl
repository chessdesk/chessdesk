## Menus

menu-game = खेल
menu-review = समीक्षा
menu-view = दृश्य
new-game = नया खेल
open-game = खेल खोलें
save-game-as = खेल इस रूप में सहेजें
export-game = खेल निर्यात करें
take-back = चाल वापस लें
resign = हार मानें
settings = सेटिंग्स
welcome-title = Chessdesk में आपका स्वागत है
welcome-intro = इसे अपने हिसाब से सजाएँ। यह सब बाद में सेटिंग्स में बदला जा सकता है।
welcome-opponents = आप किसके ख़िलाफ़ खेलना चाहते हैं?
welcome-opponents-intro = वे एजेंट चुनें जिन्हें आप इस्तेमाल करते हैं, और Chessdesk उनकी ज़रूरत की चीज़ें सेट कर देगा। वे आपके अपने खाते से खेलते हैं।
welcome-finish = सेटअप पूरा करें
quit = बंद करें
analyze-with-stockfish = Stockfish से विश्लेषण करें
review-with = { $agent } से समीक्षा करें
agent-settings = एजेंट सेटिंग्स
flip-board = बोर्ड पलटें
game-history = खेलों का इतिहास
game-panel = खेल पैनल
hide-win-likelihood = जीत की संभावना छिपाएँ
show-win-likelihood = जीत की संभावना दिखाएँ
first-move = पहली चाल
last-move = आख़िरी चाल
previous-move = पिछली चाल
next-move = अगली चाल
close-settings = सेटिंग्स बंद करें

## About

about = परिचय
help = सहायता
github-repository = GitHub रिपॉज़िटरी
check-for-updates = अपडेट देखें…
about-description = AI एजेंटों और इंजनों के साथ खेलने के लिए एक डेस्कटॉप शतरंज ऐप।
about-version = संस्करण { $version }
update-to = { $version } पर अपडेट करें
restart-to-update = अपडेट के लिए फिर से शुरू करें
update-engine = { $name } अपडेट करें
update-engines = इंजन अपडेट करें
whats-new = नया क्या है
whats-new-in = { $name } में नया क्या है
notes-more = …और { $count } अन्य
check-now = अभी देखें
checking-for-updates = अपडेट देखे जा रहे हैं
up-to-date = संस्करण { $version }, सबसे नया
update-available = संस्करण { $version } आ गया है, आपके पास { $current } है
update-ready = संस्करण { $version } तैयार है: इसे चलाने के लिए फिर से शुरू करें
release-notes = रिलीज़ नोट्स

## Window and common buttons

minimize = छोटा करें
restore = पहले जैसा करें
maximize = बड़ा करें
close = बंद करें
copy-trace = त्रुटि विवरण कॉपी करें
copied = कॉपी हो गया
report-issue = समस्या बताएँ
cancel = रद्द करें
delete = हटाएँ
export = निर्यात करें
choose = चुनें
open = खोलें
rename = नाम बदलें
install = इंस्टॉल करें

## Work under way, in the title bar

activity-thinking = { $agent } सोच रहा है
activity-reviewing = { $agent } खेल की समीक्षा कर रहा है
activity-analyzing = Stockfish से विश्लेषण, चाल { $done } / { $all }
activity-downloading = { $name } डाउनलोड हो रहा है, { $percent }%
activity-installing = { $agent } एडैप्टर इंस्टॉल हो रहा है
activity-asking-models = { $agent } से उसके मॉडल पूछे जा रहे हैं
activity-asking-efforts = { $agent } से उसके सोचने के स्तर पूछे जा रहे हैं

## Game history

move-count =
    { $count ->
        [one] 1 चाल
       *[other] { $count } चालें
    }
history-empty = आपके खेले गए खेल यहाँ दिखेंगे।
delete-title = { $game } हटाएँ?
delete-hint = यह रीसायकल बिन में चला जाएगा।
delete-hint-running = यह रुककर रीसायकल बिन में चला जाएगा।
standing-won = जीते
standing-lost = हारे
draw = ड्रॉ
versus = { $white } बनाम { $black }

## Game panel

no-game = कोई खेल नहीं
result-won = आप जीत गए
result-lost = आप हार गए
ending-checkmate = शह-मात से
ending-stalemate = स्टेलमेट से
ending-insufficient-material = अपर्याप्त मोहरों से
ending-time = समय समाप्त होने से
ending-resignation = हार मानने से
verdict-perfect = बेहतरीन
verdict-good = अच्छी
verdict-inaccuracy = अशुद्धि
verdict-mistake = ग़लती
verdict-blunder = भारी भूल
verdict-likelihood = जीत की संभावना { $before }% → { $after }%
verdict-best = सबसे अच्छी चाल { $best } थी
accuracy-percent = { $percent }% सटीकता
you = आप
move-marked = Stockfish को यहाँ बेहतर चाल मिली
tokens-used = { $total } टोकन, { $cached } कैश से
progress = { $done } / { $all }
side-white = सफ़ेद
side-black = काला
side-random = कोई भी
win-likelihood = जीत की संभावना
likelihood-source = Stockfish ने आपके कंप्यूटर पर निकाली
moves = चालें
notes = नोट्स
notes-placeholder = योजनाएँ, विचार, इस खेल से याद रखने लायक बातें
analyze = विश्लेषण करें
continue = जारी रखें
back = वापस
hint = संकेत
hint-from = { $agent } का संकेत
optimal-move = सबसे अच्छी चाल
agent-instructions = एजेंट को कैसे खेलना चाहिए
instructions-example = लंदन सिस्टम से शुरुआत करो। मुझ पर थोड़ी नरमी रखना।

## New game

opponent = प्रतिद्वंद्वी
free-options = मुफ़्त विकल्प
free-models = मुफ़्त
premium-models = प्रीमियम
install-in-settings = सेटिंग्स में इंस्टॉल करें
download-in-settings = सेटिंग्स में डाउनलोड करें
strength = ताक़त
full-strength = पूरी ताक़त
engine-elo = इंजन Elo { $elo }
maia-strength = Lichess पर लगभग { $rating } के स्तर पर खेलता है
shadowchess-strength = Lichess पर { $rating } रेटिंग वाले खिलाड़ियों की तरह खेलता है
model = मॉडल
model-hint = सस्ते मॉडल कम क्रेडिट लेते हैं
model-default = डिफ़ॉल्ट ({ $model })
model-agent-default = एजेंट का डिफ़ॉल्ट
analysis-models = समीक्षा और संकेतों के लिए मॉडल
analysis-models-hint = जब आप विश्लेषण दबाते हैं या संकेत माँगते हैं, तो हर एजेंट यही इस्तेमाल करता है।
welcome-models = समीक्षा और संकेतों के लिए मॉडल
welcome-models-intro = वह मॉडल जो हर एजेंट विश्लेषण दबाने या संकेत माँगने पर इस्तेमाल करता है। आप इसे सेटिंग्स, एजेंट में बदल सकते हैं।
models-asking = मॉडल पूछे जा रहे हैं…
efforts-asking = सोचने के स्तर पूछे जा रहे हैं…
models-unknown = इसके मॉडल पूछे नहीं जा सके: { $error }
reasoning-effort = सोचने का स्तर
effort-value = { $effort }। ज़्यादा स्तर पर ज़्यादा टोकन लगते हैं
effort-low = कम
effort-medium = मध्यम
effort-high = ज़्यादा
you-play = आप खेलेंगे
time-control = समय नियंत्रण
no-clock = घड़ी नहीं
clock-minutes = { $minutes } मिनट
clock-increment = { $minutes } मिनट + { $increment } से.
token-budget = टोकन बजट
token-budget-hint = ख़त्म होने पर खेल रुक जाता है
no-limit = कोई सीमा नहीं
budget-tokens = { $tokens } टोकन
tutoring = सिखाने वाला मोड
tutoring-hint-agent = खेलते समय एजेंट आपको हर चाल समझाता है
tutoring-hint-engine = खेलते समय Stockfish आपकी हर चाल को परखता है
starting-position = शुरुआती स्थिति
position-standard = सामान्य
position-custom = अपनी स्थिति
mode-casual = दोस्ताना
mode-casual-hint = संकेत, चाल वापसी और ट्यूटरिंग
mode-competition = प्रतियोगिता
mode-competition-hint = घड़ी के साथ, बिना मदद
mode-scenario = परिदृश्य
mode-scenario-hint = तय स्थिति से शुरू करें
coming-soon = जल्द आ रहा है
speed-bullet = बुलेट
speed-blitz = ब्लिट्ज़
speed-rapid = रैपिड
speed-classical = क्लासिकल
scenario = स्थिति
scenario-endgame = एंडगेम
scenario-opening = ओपनिंग
scenario-queen-mate = राजा और वज़ीर से मात
scenario-rook-mate = राजा और हाथी से मात
scenario-king-pawn = राजा और प्यादा
scenario-lucena = लुसेना स्थिति
scenario-philidor = फ़िलिडोर स्थिति
scenario-italian = इटैलियन गेम
scenario-sicilian = सिसिलियन डिफ़ेंस
scenario-queens-gambit = क्वीन्स गैम्बिट
scenario-french = फ़्रेंच डिफ़ेंस
scenario-own = आपकी अपनी स्थिति
scenario-own-hint = क्लिपबोर्ड से एक FEN
paste-fen = FEN चिपकाएँ
show-likelihood = खेल के दौरान जीत की संभावना दिखाएँ
show-likelihood-hint = Stockfish, आपके कंप्यूटर पर। बंद रखने से खेल निष्पक्ष रहता है
start-game = खेल शुरू करें

## Settings

page-appearance = रूप-रंग
page-board = बोर्ड
page-sound = आवाज़
page-engines = इंजन
page-agents = एजेंट
page-keys = कुंजियाँ
page-plugins = प्लगइन
page-updates = अपडेट
language = भाषा
language-system = सिस्टम ({ $language })
theme = थीम
theme-dark = Chessdesk डार्क
theme-light = Chessdesk लाइट
pieces = मोहरे
font-size = फ़ॉन्ट आकार
corner-radius = कोनों की गोलाई
pixels = { $value } px
board = बोर्ड
board-follows-theme = थीम के अनुसार
board-size = बोर्ड का आकार
board-size-value = उपलब्ध जगह का { $percent }%
animate-moves = चालों में एनिमेशन
animate-moves-hint = प्रतिद्वंद्वी की चालें और खेल को पलटते समय मोहरे सरककर अपनी जगह आते हैं
opponent-delay = प्रतिद्वंद्वी की देरी
opponent-delay-value = प्रतिद्वंद्वी की हर चाल में कम से कम { $seconds } से. लगते हैं
sound-pack = साउंड पैक
sound-off = बंद
volume = आवाज़ का स्तर
volume-value = { $percent }%
downloading = डाउनलोड हो रहा है, { $percent }%
font-needed = इस भाषा को { $font } फ़ॉन्ट चाहिए
font-download-size = { $megabytes } MB का डाउनलोड, प्रगति शीर्षक पट्टी में दिखती है
installed = इंस्टॉल है
engine-latest = इंस्टॉल है, नवीनतम ({ $version })
engine-newer = { $version } उपलब्ध है
download = डाउनलोड करें
update = अपडेट करें
choose-program = प्रोग्राम चुनें
choose-folder = फ़ोल्डर चुनें
choose-file = फ़ाइल चुनें
not-set = सेट नहीं है
maia-networks = Maia नेटवर्क
installing-adapter = एडैप्टर इंस्टॉल हो रहा है
adapter-missing = एडैप्टर इंस्टॉल नहीं है
adapter-to-install = Chessdesk इसका एडैप्टर इंस्टॉल करेगा
stockfish-offer = हमेशा शामिल: विरोधी के रूप में, संकेतों के लिए और आपकी बाज़ियों के विश्लेषण के लिए। लगभग 80 MB
shadowchess-offer = खेलने के लिए सुझाया गया इंजन। आपकी चुनी रेटिंग वाले इंसान की तरह खेलता है, गलतियों समेत। लगभग 14 MB
maia-offer = यह भी इंसानों जैसा है, पर अपनी रेटिंग से ज़्यादा मज़बूत खेलता है। चलाने के लिए lc0 डाउनलोड करता है
needs-node = Node.js चाहिए
program-not-found = { $program } नहीं मिला
plugins-folder = प्लगइन फ़ोल्डर
plugins-folder-hint = इंस्टॉल करने के लिए प्लगइन फ़ोल्डर यहाँ कॉपी करें
installed-plugins = इंस्टॉल किए गए प्लगइन
open-toml = TOML खोलें

## Why an opponent can't play

agent-needs-node = इस एजेंट को Node.js चाहिए।
agent-adapter-missing = { $agent } एडैप्टर इंस्टॉल नहीं है।
stockfish-missing = Stockfish इंस्टॉल नहीं है।
maia-missing = Maia को lc0 और उसके नेटवर्क चाहिए।
shadowchess-missing = Shadowchess का नेटवर्क डाउनलोड नहीं हुआ है।
stockfish-needed = { $feature ->
    [likelihood] जीत की संभावना के लिए Stockfish चाहिए
    [tutoring] सिखाने वाले मोड के लिए Stockfish चाहिए
   *[both] जीत की संभावना और सिखाने वाले मोड के लिए Stockfish चाहिए
}
stockfish-download-size = लगभग 80 MB, प्रगति शीर्षक पट्टी में दिखती है

## Exporting

export-location = जगह
export-pgn = विश्लेषण सहित PGN
export-pgn-moves = केवल चालों वाला PGN
export-fen = आख़िरी स्थिति का FEN

## Errors and notices

error-remember-choices = विकल्प याद नहीं रखे जा सके: { $error }
error-save-setup = खेल की सेटिंग सहेजी नहीं जा सकी: { $error }
error-save-instructions = निर्देश सहेजे नहीं जा सके: { $error }
error-save-game = खेल सहेजा नहीं जा सका: { $error }
error-delete-game = खेल हटाया नहीं जा सका: { $error }
error-export-game = खेल निर्यात नहीं हो सका: { $error }
error-save-setting = सेटिंग सहेजी नहीं जा सकी: { $error }
error-download = डाउनलोड विफल: { $error }
error-install = इंस्टॉल विफल: { $error }
error-timed-out = { $agent } ने { $minutes } मिनट में कोई चाल नहीं दी (सोचने का स्तर { $effort })। जारी रखें फिर से पूछता है; कम स्तर पर जवाब जल्दी आता है।
error-update-check = अपडेट नहीं देखे जा सके: { $error }
error-restart = फिर से शुरू नहीं हो सका: { $error }
error-review = समीक्षा: { $error }
error-no-data-folder = इस सिस्टम पर कोई डेटा फ़ोल्डर नहीं है
invalid-position = यह शुरुआती स्थिति मान्य नहीं है।
invalid-fen = क्लिपबोर्ड में कोई मान्य FEN नहीं है।
illegal-move = यहाँ यह चाल नियम के विरुद्ध है।
budget-used = { $budget } टोकन का बजट ख़त्म हो गया।
needs-stockfish = इसके लिए Stockfish चाहिए। इसे सेटिंग्स, इंजन में डाउनलोड करें।
only-own-games-resign = केवल यहाँ खेले गए खेलों में ही हार मानी जा सकती है।
agent-not-set-up = { $agent } सेट नहीं है। सेटिंग्स, एजेंट देखें।
review-empty = समीक्षा में कोई टिप्पणी नहीं जोड़ी गई।
