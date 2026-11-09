//! The models each agent offers. An agent says so only when a session starts, which takes a
//! few seconds but no tokens, so the answers are kept on disk: a list shows at once, and one
//! older than a day is asked for again in the background.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chessdesk_players::acp::{self, AgentConfig, AgentKind, Effort, Models};
use gpui::{AppContext, Context, SharedString};
use serde::{Deserialize, Serialize};

use crate::i18n::{tr, tr_with};
use crate::settings::Settings;
use crate::ui::Activity;

/// How long an agent's list is trusted before it's asked for again.
const FRESH_FOR: Duration = Duration::from_hours(24);

/// An agent's answer as kept on disk.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Saved {
    /// When it was asked, in seconds since 1970.
    asked: u64,
    /// `None` if the agent offers no choice.
    models: Option<Models>,
}

/// The saved answers, by the agent's name.
type SavedModels = BTreeMap<String, Saved>;

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

fn is_fresh(saved: &Saved, now: u64) -> bool {
    now.saturating_sub(saved.asked) < FRESH_FOR.as_secs()
}

fn cache_path() -> Option<PathBuf> {
    Some(Settings::data_dir()?.join("models.toml"))
}

/// What's kept, or nothing if there's none or it doesn't read: it's only a cache.
fn load_saved() -> SavedModels {
    cache_path()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|text| toml::from_str(&text).ok())
        .unwrap_or_default()
}

fn store_saved(saved: &SavedModels) -> Result<(), String> {
    let path = cache_path().ok_or("no data folder on this system")?;
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    }
    let text = toml::to_string(saved).map_err(|error| error.to_string())?;
    fs::write(path, text).map_err(|error| error.to_string())
}

/// A model to pick: what it shows as, and what it's asked for by, `None` being the agent's
/// own default.
pub type ModelEntry = (SharedString, Option<String>);

/// Longer lists than this get a field to type in, to narrow them.
pub const MAX_PLAIN_MODELS: usize = 5;

/// The models `offer` has to pick from, the agent's default first. `chosen` joins them if
/// it's not among them: typed, or remembered from before.
pub fn model_entries(offer: Option<&Models>, chosen: Option<&str>) -> Vec<ModelEntry> {
    let default = offer.map(|models| {
        models
            .find(&models.current)
            .map_or(models.current.as_str(), |model| model.name.as_str())
    });
    let mut entries: Vec<ModelEntry> = vec![(
        default.map_or_else(
            || tr("model-agent-default"),
            |name| tr_with!("model-default", model = name),
        ),
        None,
    )];
    entries.extend(offer.into_iter().flat_map(|models| {
        models
            .choices
            .iter()
            .map(|model| (model.name.clone().into(), Some(model.id.clone())))
    }));
    if let Some(chosen) = chosen
        && !entries.iter().any(|(_, id)| id.as_deref() == Some(chosen))
    {
        entries.push((chosen.to_owned().into(), Some(chosen.to_owned())));
    }
    entries
}

/// What's known of one agent's models.
#[derive(Clone, Debug)]
pub enum Offer {
    Asking,
    /// `None` if the agent offers no choice.
    Known(Option<Models>),
    Failed(String),
}

pub struct Catalog {
    offers: HashMap<AgentKind, Offer>,
    saved: SavedModels,
}

impl Catalog {
    /// Starts from the answers kept on disk.
    pub fn load() -> Self {
        Self {
            offers: HashMap::new(),
            saved: load_saved(),
        }
    }

    pub fn offer(&self, kind: AgentKind) -> Option<&Offer> {
        self.offers.get(&kind)
    }

    /// The models `kind` offers, once known.
    pub fn models(&self, kind: AgentKind) -> Option<&Models> {
        match self.offers.get(&kind)? {
            Offer::Known(models) => models.as_ref(),
            _ => None,
        }
    }

    /// The name of `model`, or of the agent's own default for `None`, as the agent shows it.
    pub fn model_name(&self, kind: AgentKind, model: Option<&str>) -> Option<String> {
        let models = self.models(kind)?;
        let id = model.unwrap_or(&models.current);
        let name = models.find(id).map_or(id, |found| found.name.as_str());
        Some(name.to_owned())
    }

    /// What's being asked, for the title bar.
    pub fn activity(&self) -> Option<Activity> {
        self.offers
            .iter()
            .find(|(_, offer)| matches!(offer, Offer::Asking))
            .map(|(kind, _)| Activity::new(tr_with!("activity-asking-models", agent = kind.name())))
    }

    /// Asks `kind` for its models, unless that's known or under way. A saved list shows at
    /// once, and is asked for again quietly once it's old. A failed ask is tried again: the
    /// agent may have been set up since.
    pub fn ask(&mut self, kind: AgentKind, cx: &mut Context<Self>) {
        if matches!(
            self.offers.get(&kind),
            Some(Offer::Asking | Offer::Known(_))
        ) {
            return;
        }
        let saved = self.saved.get(kind.name()).cloned();
        let quiet = saved.is_some();
        match saved {
            Some(saved) => {
                self.offers.insert(kind, Offer::Known(saved.models.clone()));
                if is_fresh(&saved, now()) {
                    return;
                }
            }
            None => {
                self.offers.insert(kind, Offer::Asking);
            }
        }
        cx.notify();
        let config = AgentConfig {
            kind,
            model: None,
            effort: Effort::Low,
            adapters: Settings::adapters_dir().unwrap_or_default(),
            reply_language: None,
        };
        cx.spawn(async move |this, cx| {
            let answer = cx
                .background_spawn(async move { acp::models(&config) })
                .await;
            this.update(cx, |this, cx| {
                match answer {
                    Ok(models) => this.remember(kind, models),
                    // The saved list stays: it's likely still right.
                    Err(_) if quiet => {}
                    Err(error) => {
                        this.offers.insert(kind, Offer::Failed(error.to_string()));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn remember(&mut self, kind: AgentKind, models: Option<Models>) {
        self.offers.insert(kind, Offer::Known(models.clone()));
        let saved = Saved {
            asked: now(),
            models,
        };
        self.saved.insert(kind.name().to_owned(), saved);
        // Not worth a word if it fails: the list is asked for again next time.
        let _ = store_saved(&self.saved);
    }
}

#[cfg(test)]
mod tests {
    use chessdesk_players::acp::Model;

    use super::*;

    #[test]
    fn saved_models_read_back() {
        let models = Models {
            option: Some("model".to_owned()),
            current: "opus".to_owned(),
            choices: vec![Model {
                id: "opus".to_owned(),
                name: "Opus 5.5".to_owned(),
                description: None,
            }],
        };
        let saved = SavedModels::from([
            (
                "Claude Code".to_owned(),
                Saved {
                    asked: 7,
                    models: Some(models),
                },
            ),
            (
                "Copilot".to_owned(),
                Saved {
                    asked: 9,
                    models: None,
                },
            ),
        ]);
        let text = toml::to_string(&saved).unwrap();
        assert_eq!(toml::from_str::<SavedModels>(&text).unwrap(), saved);
    }

    #[test]
    fn the_agents_default_comes_first_and_a_typed_model_stays() {
        let models = Models {
            option: Some("model".to_owned()),
            current: "opus".to_owned(),
            choices: vec![Model {
                id: "opus".to_owned(),
                name: "Opus 5.5".to_owned(),
                description: None,
            }],
        };
        let entries = model_entries(Some(&models), Some("my-model"));
        let ids: Vec<_> = entries.iter().map(|(_, id)| id.as_deref()).collect();
        assert_eq!(ids, [None, Some("opus"), Some("my-model")]);
        assert_eq!(entries[0].0, "Default (Opus 5.5)");
        assert_eq!(model_entries(None, None)[0].0, "The agent's default");
    }

    #[test]
    fn a_list_goes_stale_after_a_day() {
        let saved = Saved {
            asked: 1_000_000,
            models: None,
        };
        assert!(is_fresh(&saved, 1_000_000 + 60));
        assert!(!is_fresh(&saved, 1_000_000 + FRESH_FOR.as_secs()));
    }
}
