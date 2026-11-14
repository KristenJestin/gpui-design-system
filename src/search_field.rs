//! `SearchField`: a text input that filters a list under it. A search icon at its start, the
//! placeholder that says what it searches (there is no label above it), and at its end a clear
//! button that unfolds and fades in over [`PANEL`] once the field holds text, and folds away when
//! it is empty again. Escape clears the field too. Its border moves to the focus colour over
//! [`HOVER`] as the pointer enters it or the focus does.
//!
//! The text is the caller's: an `InputState` it made and styled with
//! [`follow_theme`](crate::follow_theme), as every text field of the window is.

use std::sync::Arc;

use gpui_kit::base::input::{Input, InputEvent, InputState};
use gpui_kit::{
    App, Context, ElementId, Entity, EntityId, Focusable, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Subscription,
    Window, div, px,
};

use crate::button::Button;
use crate::color::mix;
use crate::icons::{Icon, icon};
use crate::interact::{fade, hoverable};
use crate::motion::{HOVER, PANEL};
use crate::theme::palette;

/// The height of the field, as a select's.
const HEIGHT: f32 = 34.;
/// The width of the clear button once it has unfolded: the icon button's.
const CLEAR: f32 = 28.;
/// The room between the text and the clear button.
const GAP: f32 = 8.;

/// Redraws the view that draws the field when its text or its focus changes (an `InputState`
/// notifies only itself, and a keyed state notifies the view that drew it). The input notifies on
/// every blink of its caret too, which must not redraw the caller twice a second: only a change
/// of the text or of the focus is passed on.
struct Watch {
    input: EntityId,
    text: SharedString,
    focused: bool,
    _changes: [Subscription; 2],
}

impl Watch {
    fn new(input: &Entity<InputState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            input: input.entity_id(),
            text: input.read(cx).value(),
            focused: input.focus_handle(cx).contains_focused(window, cx),
            _changes: [
                cx.observe_in(input, window, Self::check),
                cx.subscribe_in(input, window, |watch, input, _: &InputEvent, window, cx| {
                    watch.check(input.clone(), window, cx)
                }),
            ],
        }
    }

    fn check(&mut self, input: Entity<InputState>, window: &mut Window, cx: &mut Context<Self>) {
        let text = input.read(cx).value();
        let focused = input.focus_handle(cx).contains_focused(window, cx);
        if text != self.text || focused != self.focused {
            self.text = text;
            self.focused = focused;
            cx.notify();
        }
    }
}

#[derive(IntoElement)]
pub struct SearchField {
    id: ElementId,
    input: Entity<InputState>,
    label: SharedString,
    placeholder: Option<SharedString>,
}

impl SearchField {
    /// The field `id`, its clear button `<id>-clear`. `label` is the accessible name: nothing is
    /// drawn above the field, its placeholder says what it searches.
    pub fn new(
        id: impl Into<ElementId>,
        input: &Entity<InputState>,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            input: input.clone(),
            label: label.into(),
            placeholder: None,
        }
    }

    /// What it searches ("Search 35 actions by name or description"); it follows a count that
    /// changes.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }
}

fn part(id: &ElementId, name: &'static str) -> ElementId {
    match id {
        ElementId::Name(text) => ElementId::Name(format!("{text}-{name}").into()),
        other => ElementId::NamedChild(Arc::new(other.clone()), name.into()),
    }
}

impl RenderOnce for SearchField {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let watch = window.use_keyed_state(part(&self.id, "watch"), cx, |window, cx| {
            Watch::new(&self.input, window, cx)
        });
        // The caller may hand the same id another input: it is the one to watch from now on.
        if watch.read(cx).input != self.input.entity_id() {
            watch.update(cx, |watch, cx| *watch = Watch::new(&self.input, window, cx));
        }
        if let Some(placeholder) = self.placeholder
            && self.input.read(cx).presentation().placeholder() != &placeholder
        {
            self.input.update(cx, |input, cx| {
                input.set_placeholder(placeholder, window, cx)
            });
        }
        let filled = !self.input.read(cx).value().is_empty();
        let focused = self.input.focus_handle(cx).contains_focused(window, cx);
        let ring = fade(part(&self.id, "focus"), focused, HOVER, window, cx);
        // Present while there is text, and while it folds away after it has gone.
        let shown = fade(part(&self.id, "clear-shown"), filled, PANEL, window, cx);
        crate::seen::note(&part(&self.id, "clear"), shown);
        let clear = (shown > 0.0).then(|| {
            let input = self.input.clone();
            // The gap before the button is part of what unfolds, so that it grows and shrinks
            // with it instead of appearing at once.
            div()
                .flex_none()
                .overflow_hidden()
                .w(px((GAP + CLEAR) * shown))
                .pl(px(GAP * shown))
                .opacity(shown)
                .child(
                    Button::icon(part(&self.id, "clear"), Icon::Close, "Clear")
                        .ghost()
                        .on_click(move |_, window, cx| {
                            input.update(cx, |input, cx| input.clean(window, cx));
                        }),
                )
        });
        let input = self.input.clone();
        let field = div()
            .id(self.id.clone())
            .aria_label(self.label)
            .w_full()
            .h(px(HEIGHT))
            .pl(px(10.))
            .pr(px(3.))
            .rounded(px(8.))
            .bg(p.field)
            .border_1()
            .flex()
            .items_center()
            .text_size(px(13.))
            .text_color(p.text)
            .on_key_down(move |event, window, cx| {
                // Escape leaves a dialog around the field when there is nothing to clear.
                if event.keystroke.key == "escape" && !input.read(cx).value().is_empty() {
                    cx.stop_propagation();
                    input.update(cx, |input, cx| input.clean(window, cx));
                }
            })
            .child(
                icon(Icon::Search)
                    .size(px(13.))
                    .mr(px(8.))
                    .text_color(p.faint),
            )
            .child(div().flex_1().min_w_0().child(Input::new(&self.input)))
            .children(clear);
        let ring_id = part(&self.id, "ring");
        hoverable(field, move |d, hover| {
            let on = hover.max(ring);
            crate::seen::note(&ring_id, on);
            d.border_color(mix(p.field_line, p.focus_line, on))
        })
    }
}
