//! The models each agent offers, asked for once per run: an agent says so only when a session
//! starts, which takes a few seconds but no tokens.

use std::collections::HashMap;

use chessdesk_players::acp::{self, AgentConfig, AgentKind, Effort, Models};
use gpui::{AppContext, Context, SharedString};

use crate::i18n::tr_with;
use crate::settings::Settings;

/// What's known of one agent's models.
#[derive(Clone, Debug)]
pub enum Offer {
    Asking,
    /// `None` if the agent offers no choice.
    Known(Option<Models>),
    Failed(String),
}

#[derive(Default)]
pub struct Catalog {
    offers: HashMap<AgentKind, Offer>,
}

impl Catalog {
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
    pub fn activity(&self) -> Option<SharedString> {
        self.offers
            .iter()
            .find(|(_, offer)| matches!(offer, Offer::Asking))
            .map(|(kind, _)| tr_with!("activity-asking-models", agent = kind.name()))
    }

    /// Asks `kind` for its models, unless that's known or under way. A failed ask is tried
    /// again: the agent may have been set up since.
    pub fn ask(&mut self, kind: AgentKind, cx: &mut Context<Self>) {
        if matches!(
            self.offers.get(&kind),
            Some(Offer::Asking | Offer::Known(_))
        ) {
            return;
        }
        self.offers.insert(kind, Offer::Asking);
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
                let offer = match answer {
                    Ok(models) => Offer::Known(models),
                    Err(error) => Offer::Failed(error.to_string()),
                };
                this.offers.insert(kind, offer);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}
