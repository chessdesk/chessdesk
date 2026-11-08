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
welcome-start = चलिए खेलें
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
hint = संकेत
optimal-move = सबसे अच्छी चाल
agent-instructions = एजेंट को कैसे खेलना चाहिए
instructions-example = लंदन सिस्टम से शुरुआत करो। मुझ पर थोड़ी नरमी रखना।

## New game

opponent = प्रतिद्वंद्वी
not-installed = { $name } (इंस्टॉल नहीं है)
install-in-settings = सेटिंग्स में इंस्टॉल करें
download-in-settings = सेटिंग्स में डाउनलोड करें
strength = ताक़त
full-strength = पूरी ताक़त
engine-elo = इंजन Elo { $elo }
maia-strength = Lichess पर लगभग { $rating } के स्तर पर खेलता है
model = मॉडल
model-hint = सस्ते मॉडल कम क्रेडिट लेते हैं
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
download = डाउनलोड करें
update = अपडेट करें
choose-program = प्रोग्राम चुनें
choose-folder = फ़ोल्डर चुनें
not-set = सेट नहीं है
maia-networks = Maia नेटवर्क
installing-adapter = एडैप्टर इंस्टॉल हो रहा है
adapter-missing = एडैप्टर इंस्टॉल नहीं है
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
