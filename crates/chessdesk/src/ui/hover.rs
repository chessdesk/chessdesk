//! A hover that fades in and out. GPUI's own `hover` style switches at once, so controls
//! take this as their first child instead: it covers its parent, notices the pointer
//! itself, and paints the parent's hover colour behind everything else in it.

use std::time::{Duration, Instant};

use super::Transition;

use gpui::{
    App, BorderStyle, Bounds, DispatchPhase, Edges, Element, ElementId, GlobalElementId, Hitbox,
    HitboxBehavior, Hsla, InspectorElementId, IntoElement, LayoutId, Length, MouseMoveEvent,
    Pixels, Position, Style, Window, px, quad, transparent_black,
};

/// How long the colour takes to come and go: quick enough to follow the pointer.
const FADE: Duration = Duration::from_millis(120);

/// A parent's hover colour, fading in and out with the pointer over the parent.
pub struct HoverFill {
    color: Hsla,
    radius: Pixels,
}

/// Fills its parent with `color` while the pointer is over it, rounded like the parent.
pub fn hover_fill(color: Hsla, radius: Pixels) -> HoverFill {
    HoverFill { color, radius }
}

impl IntoElement for HoverFill {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for HoverFill {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;

    /// Its state lives under the parent's id, so one name serves every control.
    fn id(&self) -> Option<ElementId> {
        Some("hover-fill".into())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let edge: Length = px(0.).into();
        let style = Style {
            position: Position::Absolute,
            inset: Edges::all(edge),
            ..Style::default()
        };
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        (): &mut Self::RequestLayoutState,
        window: &mut Window,
        _: &mut App,
    ) -> Self::PrepaintState {
        // Lets the pointer through to the parent and everything else in it.
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        (): &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        _: &mut App,
    ) {
        let Some(id) = id else {
            return;
        };
        let hovered = hitbox.is_hovered(window);
        let now = Instant::now();
        // Settled where the pointer is at first, so that nothing fades as a control shows.
        let fade = window.with_element_state(id, |fade: Option<Transition>, _| {
            let mut fade = fade.unwrap_or_else(|| Transition::settled(hovered, FADE, now));
            fade.set(hovered, now);
            (fade, fade)
        });
        if fade.is_moving(now) {
            window.request_animation_frame();
        }
        let shown = fade.value(now);
        if shown > 0. {
            window.paint_quad(quad(
                bounds,
                self.radius,
                self.color.opacity(self.color.a * shown),
                px(0.),
                transparent_black(),
                BorderStyle::default(),
            ));
        }
        // Draws again once the pointer comes or goes.
        let hitbox = hitbox.clone();
        let view = window.current_view();
        window.on_mouse_event(move |_: &MouseMoveEvent, phase, window, cx| {
            if phase == DispatchPhase::Capture && hitbox.is_hovered(window) != hovered {
                cx.notify(view);
            }
        });
    }
}
