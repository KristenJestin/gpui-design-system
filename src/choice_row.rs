//! `ChoiceRow`: one option of a list the user picks from, as a row: a mark (round for one choice
//! among several, square for any number of them), the label, and under it a description on as
//! many lines as it needs. The whole row is the target. Its hover fades over
//! [`HOVER`](crate::motion::HOVER); when it is chosen its fill and border fade in and its mark
//! fills, its dot scaling in or its tick growing, over [`PANEL`](crate::motion::PANEL).
//!
//! A row may hold a field ([`ChoiceRow::field`]): once chosen, the field unfolds and fades in to
//! the right of the label, which wraps rather than squeeze it away; the label drops onto the
//! field's first line with the mark and the row's height follows, over
//! [`PANEL`](crate::motion::PANEL), and the label's words change with the fade of
//! [`changing`](crate::reveal::changing), over [`HOVER`](crate::motion::HOVER).

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, RenderOnce, SharedString, StatefulInteractiveElement, Styled,
    TestSupportExt, Window, div, px,
};

use crate::color::mix;
use crate::icons::{Icon, icon};
use crate::interact::selectable;
use crate::motion::Motion;
use crate::reveal::{Shown, changing, drive, shown_frame};
use crate::theme::palette;

/// The room between the label and its field.
const FIELD_GAP: f32 = 8.;
/// The least width the field keeps: a long label wraps before it squeezes the field past this.
const FIELD_MIN: f32 = 120.;

type OnClick = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Round,
    Square,
}

#[derive(IntoElement)]
pub struct ChoiceRow {
    id: ElementId,
    label: SharedString,
    description: Option<SharedString>,
    shape: Shape,
    chosen: bool,
    faint: bool,
    field: Option<(SharedString, AnyElement)>,
    field_inset: Pixels,
    on_click: Option<OnClick>,
}

impl ChoiceRow {
    /// A row with a round mark. Its parts are `<id>-label`, `<id>-line` (the description),
    /// `<id>-mark` and `<id>-dot` (round) or `<id>-tick` (square).
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
            shape: Shape::Round,
            chosen: false,
            faint: false,
            field: None,
            field_inset: px(0.),
            on_click: None,
        }
    }

    /// What the choice means, under its label.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// A square mark with a tick: the row is one of several that may all be chosen.
    pub fn square(mut self) -> Self {
        self.shape = Shape::Square;
        self
    }

    pub fn chosen(mut self, chosen: bool) -> Self {
        self.chosen = chosen;
        self
    }

    /// A faint label in the regular weight: the way out of the choices ("Something else…").
    pub fn faint(mut self) -> Self {
        self.faint = true;
        self
    }

    /// A field the row holds once it is chosen, to the right of its label: it unfolds and fades
    /// in and takes the rest of the row's width (at least 120 px: a long label wraps instead), the
    /// label shows `words` meanwhile (the label again once the field is gone), and a click inside
    /// the field does not toggle the row. Parts: `<id>-field` (the fold) and `<id>-slot` (what
    /// holds the field).
    pub fn field(mut self, words: impl Into<SharedString>, field: impl IntoElement) -> Self {
        self.field = Some((words.into(), field.into_any_element()));
        self
    }

    /// How far below the top of the field its first line of text starts (its padding and border):
    /// the label and the mark drop by that much, over [`PANEL`](crate::motion::PANEL), to sit on
    /// that line. 0 when the field's text starts at its top, which is the default.
    pub fn field_inset(mut self, inset: Pixels) -> Self {
        self.field_inset = inset;
        self
    }

    pub fn on_click(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for ChoiceRow {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let part = |part: &str| match &self.id {
            ElementId::Name(id) => ElementId::Name(format!("{id}-{part}").into()),
            other => ElementId::NamedChild(Arc::new(other.clone()), part.to_owned().into()),
        };
        let (mark_id, glyph_id) = (
            part("mark"),
            part(match self.shape {
                Shape::Round => "dot",
                Shape::Square => "tick",
            }),
        );
        let mut row = div()
            .id(self.id.clone())
            .aria_label(self.label.clone())
            .aria_selected(self.chosen)
            .w_full()
            .flex()
            .items_start()
            .gap(px(10.))
            .px(px(10.))
            .py(px(8.))
            .rounded(px(8.))
            .border_1()
            .cursor_pointer();
        if let Some(on_click) = self.on_click.clone() {
            row = row.on_click(move |_, window, cx| on_click(window, cx));
        }
        let faint = self.faint;
        let label_of = move |id: ElementId, text: SharedString| {
            div()
                .id(id)
                .text_size(px(14.))
                .line_height(px(20.))
                .when_faint(faint, p.mute, p.text)
                .child(text)
                .test_support()
        };
        let label_of = move |id: ElementId, text: SharedString, said: bool| {
            let label = label_of(id, text.clone());
            if said { label.aria_label(text) } else { label }
        };
        // How far the label, and the mark with it, has dropped onto the field's first line.
        let mut drop = 0.;
        let top = match self.field {
            None => label_of(part("label"), self.label.clone(), false).into_any_element(),
            Some((words, field)) => {
                let open = self.chosen;
                let key = ElementId::NamedChild(Arc::new(part("field")), "presence".into());
                let state = window.use_keyed_state(key, cx, move |_, _| {
                    Shown::new(Motion::settled(if open { 1.0 } else { 0.0 }))
                });
                let at = drive(&state, open, window, cx);
                drop = f32::from(self.field_inset) * at.t;
                crate::seen::note(&part("field"), at.t);
                let slot = div()
                    .id(part("slot"))
                    .w_full()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(field)
                    .test_support();
                let shown = shown_frame(
                    part("field"),
                    open,
                    false,
                    &at,
                    0.,
                    0.,
                    slot.into_any_element(),
                );
                let said = if open { words } else { self.label.clone() };
                let label_id = part("label");
                div()
                    .flex()
                    .items_start()
                    .gap(px(FIELD_GAP))
                    .child(div().min_w_0().mt(px(drop)).child(changing(
                        part("said"),
                        said,
                        move |text| label_of(label_id, text, true).into_any_element(),
                    )))
                    .child(div().flex_1().min_w(px(FIELD_MIN)).child(shown))
                    .into_any_element()
            }
        };
        let line = self.description.clone().map(|description| {
            div()
                .id(part("line"))
                .aria_label(description.clone())
                .text_size(px(13.))
                .line_height(px(18.))
                .text_color(p.mute)
                .child(description)
                .test_support()
        });
        let words = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .child(top)
            .children(line);
        let shape = self.shape;
        selectable(row, self.chosen, move |d, hover, on| {
            let glyph = div().id(glyph_id);
            let glyph = match shape {
                Shape::Round => glyph
                    .size(px(6. * on))
                    .rounded_full()
                    .bg(p.on_acc)
                    .test_support(),
                Shape::Square => glyph
                    .size(px(10. * on))
                    .child(icon(Icon::Check).size(px(10. * on)).text_color(p.on_acc))
                    .test_support(),
            };
            let mark = div()
                .id(mark_id)
                .mt(px(2. + drop))
                .size(px(16.))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .border(px(1.5))
                .border_color(mix(p.line2, p.acc, on))
                .bg(mix(p.field, p.acc, on))
                .when_round(shape == Shape::Round)
                .child(glyph)
                .test_support();
            d.bg(mix(p.hover.opacity(hover), p.bg, on))
                .border_color(p.line2.opacity(on))
                .child(mark)
                .child(words)
        })
    }
}

/// The two small conveniences the row needs on a styled element.
trait RowStyle: Styled + Sized {
    fn when_round(self, round: bool) -> Self {
        if round {
            self.rounded_full()
        } else {
            self.rounded(px(4.))
        }
    }

    fn when_faint(self, faint: bool, faint_ink: gpui_kit::Hsla, ink: gpui_kit::Hsla) -> Self {
        if faint {
            self.text_color(faint_ink)
        } else {
            self.text_color(ink).font_weight(FontWeight::MEDIUM)
        }
    }
}

impl<T: Styled + Sized> RowStyle for T {}
