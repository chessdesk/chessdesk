//! Picking one of an agent's models, for a settings row: a select, or for a long list or none
//! at all, a field to type in. Says what's picked as [`Picked`]; the row keeps it.

use chessdesk_players::acp::AgentKind;
use gpui::{
    AnyElement, App, Context, Entity, EventEmitter, MouseButton, SharedString, Subscription,
    Window, div, prelude::*,
};

use crate::combobox::{self, Combobox};
use crate::i18n::{tr, tr_with};
use crate::models::{Catalog, MAX_PLAIN_MODELS, ModelEntry, Offer, model_entries};
use crate::settings::Settings;
use crate::ui::{self, Logo};

/// The model picked, `None` being the agent's own default.
pub struct Picked(pub Option<String>);

pub struct ModelPicker {
    kind: AgentKind,
    chosen: Option<String>,
    catalog: Entity<Catalog>,
    /// Whether the plain list is open.
    open: bool,
    /// The list to type in, while it's open.
    combobox: Option<(Entity<Combobox>, Subscription)>,
    _catalog_changes: Subscription,
}

impl EventEmitter<Picked> for ModelPicker {}

impl ModelPicker {
    /// A picker for `kind`'s models, on `chosen`. Asks the agent for its models if that's not
    /// known: a few seconds, no tokens.
    pub fn new(
        kind: AgentKind,
        chosen: Option<String>,
        catalog: Entity<Catalog>,
        cx: &mut Context<Self>,
    ) -> Self {
        let changes = cx.observe(&catalog, |_, _, cx| cx.notify());
        catalog.update(cx, |catalog, cx| catalog.ask(kind, cx));
        Self {
            kind,
            chosen,
            catalog,
            open: false,
            combobox: None,
            _catalog_changes: changes,
        }
    }

    /// What the row says under its label: why the models are unknown, if they are.
    pub fn hint(&self, cx: &App) -> Option<SharedString> {
        match self.catalog.read(cx).offer(self.kind)? {
            Offer::Failed(error) => Some(tr_with!("models-unknown", error = error.as_str())),
            _ => None,
        }
    }

    fn pick(&mut self, model: Option<String>, cx: &mut Context<Self>) {
        self.open = false;
        self.combobox = None;
        self.chosen.clone_from(&model);
        cx.emit(Picked(model));
        cx.notify();
    }

    fn open_combobox(
        &mut self,
        entries: Vec<ModelEntry>,
        current: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let labels = entries.iter().map(|(label, _)| label.clone()).collect();
        let combobox = cx.new(|cx| Combobox::new(current, labels, window, cx));
        let subscription = cx.subscribe(&combobox, move |this, _, event, cx| match event {
            combobox::Event::Picked(place) => this.pick(entries[*place].1.clone(), cx),
            combobox::Event::Typed(model) => this.pick(Some(model.clone()), cx),
            combobox::Event::Dismissed => {
                this.combobox = None;
                cx.notify();
            }
        });
        self.combobox = Some((combobox, subscription));
        cx.notify();
    }

    fn list(
        entries: &[ModelEntry],
        current: usize,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let settings = Settings::get(cx);
        let rows = entries.iter().enumerate().map(|(index, (label, id))| {
            let id = id.clone();
            ui::list_row(("model-choice", index), index == current, &settings)
                .text_color(settings.colors.text)
                .child(div().truncate().child(label.clone()))
                .on_click(cx.listener(move |this, _, _, cx| this.pick(id.clone(), cx)))
        });
        ui::select_dropdown(
            ui::popover(window, &settings)
                .id("model-list")
                .occlude()
                .w(ui::SELECT_WIDTH)
                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    this.open = false;
                    cx.notify();
                }))
                .children(rows),
        )
        .into_any_element()
    }
}

/// An agent's row with its picker, under its logo and name, as Settings and the welcome show it.
pub fn row(kind: AgentKind, picker: &Entity<ModelPicker>, cx: &App) -> AnyElement {
    let settings = Settings::get(cx);
    let hint = picker.read(cx).hint(cx);
    ui::logo_setting_row(Logo::of_agent(kind), kind.name(), hint, &settings)
        .child(picker.clone())
        .into_any_element()
}

impl Render for ModelPicker {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = Settings::get(cx);
        if let Some((combobox, _)) = &self.combobox {
            return div().child(combobox.clone());
        }
        let models = match self.catalog.read(cx).offer(self.kind) {
            None | Some(Offer::Asking) => {
                let asking =
                    ui::select_button("model", tr("models-asking"), None, false, &settings)
                        .cursor_default();
                return div().child(asking);
            }
            Some(Offer::Failed(_)) => None,
            Some(Offer::Known(models)) => models.clone(),
        };
        let entries = model_entries(models.as_ref(), self.chosen.as_deref());
        let current = entries
            .iter()
            .position(|(_, id)| *id == self.chosen)
            .unwrap_or(0);
        let label = entries[current].0.clone();
        let listed = models.as_ref().map_or(0, |models| models.choices.len());
        let typed = models.is_some() && (listed > MAX_PLAIN_MODELS || listed == 0);
        let open = self.open;
        let button = ui::select_button("model", label, None, open, &settings);
        if typed {
            let button = button.on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, window, cx| {
                    this.open_combobox(entries.clone(), current, window, cx);
                }),
            );
            return div().child(button);
        }
        let list = open.then(|| Self::list(&entries, current, window, cx));
        let button = button.on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _, _, cx| {
                this.open = !open;
                cx.notify();
            }),
        );
        div().child(button).children(list)
    }
}
