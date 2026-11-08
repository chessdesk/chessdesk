//! The interface's words. English is built in; a language plugin brings a Project Fluent
//! file, and whatever it leaves out shows in English, so a translation can start small.

use std::fs;
use std::iter;
use std::sync::{Arc, LazyLock, RwLock};

use chessdesk_core::plugins::Language;
use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use gpui::SharedString;
use unic_langid::LanguageIdentifier;

const ENGLISH_SOURCE: &str = include_str!("../assets/en.ftl");
/// In `config.toml`: whichever language the system runs in, if there is a plugin for it.
pub const SYSTEM: &str = "System";
/// The language that needs no plugin.
pub const ENGLISH: &str = "English";

type Bundle = FluentBundle<FluentResource>;

/// Reads a Fluent file. Anything it can't parse is skipped and named in the error, so that
/// one bad line costs one message and not the whole language.
fn bundle(locale: LanguageIdentifier, source: String) -> (Bundle, Option<String>) {
    let (resource, problems) = match FluentResource::try_new(source) {
        Ok(resource) => (resource, None),
        Err((resource, errors)) => (resource, Some(format!("{} bad lines", errors.len()))),
    };
    let mut bundle = FluentBundle::new_concurrent(vec![locale]);
    // Fluent would otherwise wrap arguments in direction marks, which fonts draw as boxes.
    bundle.set_use_isolating(false);
    // Only a second definition of a message fails here, and the first one stands.
    let _ = bundle.add_resource(resource);
    (bundle, problems)
}

fn english() -> Bundle {
    bundle(LanguageIdentifier::default(), ENGLISH_SOURCE.to_owned()).0
}

struct Messages {
    /// The plugin's name for its language, and what it translates.
    chosen: Option<(String, Bundle)>,
    english: Bundle,
}

impl Messages {
    fn format(&self, id: &str, args: Option<&FluentArgs>) -> SharedString {
        let chosen = self.chosen.as_ref().map(|(_, bundle)| bundle);
        for bundle in chosen.into_iter().chain(iter::once(&self.english)) {
            if let Some(pattern) = bundle.get_message(id).and_then(|message| message.value()) {
                let text = bundle.format_pattern(pattern, args, &mut Vec::new());
                return text.into_owned().into();
            }
        }
        id.to_owned().into()
    }
}

static CURRENT: LazyLock<RwLock<Arc<Messages>>> = LazyLock::new(|| {
    RwLock::new(Arc::new(Messages {
        chosen: None,
        english: english(),
    }))
});

fn current() -> Arc<Messages> {
    CURRENT
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// A message by its id.
pub fn tr(id: &str) -> SharedString {
    current().format(id, None)
}

/// A message with arguments, which it refers to by name.
pub fn format(id: &str, args: &FluentArgs) -> SharedString {
    current().format(id, Some(args))
}

/// `tr_with!("move-count", count = 3)`.
macro_rules! tr_with {
    ($id:literal, $($name:ident = $value:expr),+ $(,)?) => {{
        let mut args = fluent_bundle::FluentArgs::new();
        $(args.set(stringify!($name), $value);)+
        $crate::i18n::format($id, &args)
    }};
}
pub(crate) use tr_with;

/// The plugin for a system locale such as `de-DE`, going by the language alone.
fn matching<'a>(locale: &str, languages: &'a [Language]) -> Option<&'a Language> {
    let primary = |tag: &str| {
        let subtag = tag.split(['-', '_']).next().unwrap_or(tag);
        subtag.to_ascii_lowercase()
    };
    let wanted = primary(locale);
    languages
        .iter()
        .find(|language| primary(&language.locale) == wanted)
}

/// The language `SYSTEM` comes to on this system: its own if a plugin has it, or English.
pub fn detected(languages: &[Language]) -> String {
    let language =
        sys_locale::get_locale().and_then(|locale| matching(&locale, languages).cloned());
    language.map_or_else(|| ENGLISH.to_owned(), |language| language.name)
}

/// The language `name` stands for, `None` for English.
pub fn chosen<'a>(name: &str, languages: &'a [Language]) -> Result<Option<&'a Language>, String> {
    match name {
        ENGLISH => Ok(None),
        SYSTEM => Ok(sys_locale::get_locale().and_then(|locale| matching(&locale, languages))),
        name => languages
            .iter()
            .find(|language| language.name == name)
            .map(Some)
            .ok_or_else(|| "no such language".to_owned()),
    }
}

fn load(language: &Language) -> Result<((String, Bundle), Option<String>), String> {
    let locale = language
        .locale
        .parse()
        .map_err(|_| format!("{} is not a locale", language.locale))?;
    let source = fs::read_to_string(&language.file).map_err(|error| error.to_string())?;
    let (bundle, problems) = bundle(locale, source);
    Ok(((language.name.clone(), bundle), problems))
}

/// Switches the interface to the language `config.toml` names. On failure it stays in
/// English, and on a partly broken file it uses what it could read: either way the error
/// says what's wrong.
pub fn use_language(name: &str, languages: &[Language]) -> Result<(), String> {
    let (chosen, problem) = match chosen(name, languages) {
        Ok(Some(language)) => match load(language) {
            Ok((chosen, problems)) => (Some(chosen), problems),
            Err(error) => (None, Some(error)),
        },
        Ok(None) => (None, None),
        Err(error) => (None, Some(error)),
    };
    let messages = Arc::new(Messages {
        chosen,
        english: english(),
    });
    *CURRENT
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = messages;
    problem.map_or(Ok(()), Err)
}

/// The language agents should comment in, unless that's English, which they default to.
pub fn reply_language() -> Option<String> {
    current().chosen.as_ref().map(|(name, _)| name.clone())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    use fluent_bundle::FluentArgs;

    use super::*;
    use crate::bundled;

    fn messages(source: &str) -> Messages {
        let locale = "de".parse().unwrap();
        Messages {
            chosen: Some(("Deutsch".to_owned(), bundle(locale, source.to_owned()).0)),
            english: english(),
        }
    }

    /// The ids a Fluent file defines: lines that start a message, not continue one.
    fn ids(source: &str) -> BTreeSet<&str> {
        source
            .lines()
            .filter(|line| line.starts_with(|first: char| first.is_ascii_lowercase()))
            .filter_map(|line| line.split_once(" =").map(|(id, _)| id))
            .collect()
    }

    /// The `$arguments` of each message, as a set per id.
    fn arguments(source: &str) -> Vec<(String, BTreeSet<String>)> {
        let mut found: Vec<(String, BTreeSet<String>)> = Vec::new();
        for line in source.lines() {
            if line.starts_with(|first: char| first.is_ascii_lowercase())
                && let Some((id, _)) = line.split_once(" =")
            {
                found.push((id.to_owned(), BTreeSet::new()));
            }
            let Some((_, used)) = found.last_mut() else {
                continue;
            };
            for part in line.split('$').skip(1) {
                let name = part.split(|c: char| !c.is_alphanumeric() && c != '_');
                used.extend(name.take(1).map(str::to_owned));
            }
        }
        found.sort();
        found
    }

    fn sources(folder: &Path, found: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(folder).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                sources(&path, found);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                found.push(path);
            }
        }
    }

    #[test]
    fn english_reads_without_a_bad_line() {
        let (_, problems) = bundle(LanguageIdentifier::default(), ENGLISH_SOURCE.to_owned());
        assert_eq!(problems, None);
    }

    #[test]
    fn what_a_translation_leaves_out_shows_in_english() {
        let messages = messages("resign = Aufgeben\n");
        assert_eq!(messages.format("resign", None), "Aufgeben");
        assert_eq!(messages.format("quit", None), "Quit");
        assert_eq!(messages.format("no-such-id", None), "no-such-id");
    }

    #[test]
    fn plurals_follow_the_language() {
        let messages = messages(
            "move-count = { $count ->\n    [one] ein Zug\n   *[other] { $count } Züge\n }\n",
        );
        let count = |count: u32| {
            let mut args = FluentArgs::new();
            args.set("count", count);
            messages.format("move-count", Some(&args))
        };
        assert_eq!(count(1), "ein Zug");
        assert_eq!(count(3), "3 Züge");
    }

    #[test]
    fn a_bad_line_costs_only_itself() {
        let (bundle, problems) = bundle("de".parse().unwrap(), "= nonsense\nquit = Ende\n".into());
        assert!(problems.is_some());
        assert!(bundle.has_message("quit"));
    }

    #[test]
    fn arguments_reach_the_text_without_direction_marks() {
        let messages = messages("review-with = Mit { $agent } prüfen\n");
        let mut args = FluentArgs::new();
        args.set("agent", "Codex");
        assert_eq!(
            messages.format("review-with", Some(&args)),
            "Mit Codex prüfen"
        );
    }

    #[test]
    fn a_system_locale_finds_its_language() {
        let language = |locale: &str| Language {
            name: locale.to_owned(),
            plugin: String::new(),
            license: String::new(),
            description: None,
            locale: locale.to_owned(),
            file: PathBuf::new(),
            font: None,
        };
        let languages = [language("de"), language("pt-BR")];
        assert_eq!(matching("de-DE", &languages).unwrap().locale, "de");
        assert_eq!(matching("pt_PT", &languages).unwrap().locale, "pt-BR");
        assert!(matching("fi-FI", &languages).is_none());
    }

    #[test]
    fn every_message_is_used_somewhere() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        sources(&root, &mut files);
        let code: String = files
            .iter()
            .map(|path| fs::read_to_string(path).unwrap())
            .collect();
        for id in ids(ENGLISH_SOURCE) {
            assert!(code.contains(&format!("\"{id}\"")), "{id} is unused");
        }
    }

    #[test]
    fn bundled_translations_match_english() {
        let english = arguments(ENGLISH_SOURCE);
        let translations = bundled::translations();
        assert_eq!(translations.len(), 5);
        for (path, source) in translations {
            let (_, problems) = bundle("de".parse().unwrap(), source.to_owned());
            assert_eq!(problems, None, "{path}");
            assert_eq!(arguments(source), english, "{path}");
        }
    }
}
