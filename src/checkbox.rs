//! `Checkbox`: a square box with three states, checked, mixed and empty, for a row of a list and
//! for "select all". Its mark draws in over [`HOVER`]: the tick when it is checked, a bar when it
//! is mixed (some of what it stands for is chosen), nothing when it is empty; the fill follows
//! over [`PANEL`]. A click asks for the next state: checked from empty and from mixed, empty from
//! checked. It is clickable on its own, with its label drawn beside it ([`Checkbox::labelled`]),
//! or inside a row that gives one.

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};

use crate::color::mix;
use crate::icons::{Icon, icon};
use crate::interact::{fade, hoverable};
use crate::motion::{HOVER, PANEL};
use crate::theme::palette;

type OnChange = Rc<dyn Fn(CheckState, &mut Window, &mut App) + 'static>;

/// The side of the box.
const BOX: f32 = 16.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckState {
    Checked,
    /// Some of what the box stands for is chosen: "select all" over a list with a few rows ticked.
    Mixed,
    Empty,
}

impl CheckState {
    /// What a click on a box in this state asks for.
    pub fn next(self) -> Self {
        match self {
            Self::Checked => Self::Empty,
            Self::Mixed | Self::Empty => Self::Checked,
        }
    }
}

#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    state: CheckState,
    label: SharedString,
    labelled: bool,
    on_change: Option<OnChange>,
}

impl Checkbox {
    /// The box `id`, its mark `<id>-mark`, its label `<id>-label` when it is drawn. `label` is its
    /// accessible name; a mixed box says so as its value.
    pub fn new(
        id: impl Into<ElementId>,
        state: CheckState,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            state,
            label: label.into(),
            labelled: false,
            on_change: None,
        }
    }

    /// Draws the label beside the box; a click on either is the box's. Without it the box is
    /// alone, in a row that says what it stands for.
    pub fn labelled(mut self) -> Self {
        self.labelled = true;
        self
    }

    /// Called with the state a click asks for ([`CheckState::next`]); the state is the owner's.
    pub fn on_change(mut self, f: impl Fn(CheckState, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

fn part(id: &ElementId, name: &'static str) -> ElementId {
    match id {
        ElementId::Name(text) => ElementId::Name(format!("{text}-{name}").into()),
        other => ElementId::NamedChild(Arc::new(other.clone()), name.into()),
    }
}

impl RenderOnce for Checkbox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        // The mark is how far it is drawn at all; the bar is how far of it is the mixed one.
        let on = fade(
            part(&self.id, "mark"),
            self.state != CheckState::Empty,
            HOVER,
            window,
            cx,
        );
        let bar = fade(
            part(&self.id, "bar"),
            self.state == CheckState::Mixed,
            HOVER,
            window,
            cx,
        );
        let fill = fade(
            part(&self.id, "fill"),
            self.state != CheckState::Empty,
            PANEL,
            window,
            cx,
        );
        crate::seen::note(&part(&self.id, "mark"), on);
        let mark_id = part(&self.id, "mark");
        let hover_id = part(&self.id, "hover");
        let state = self.state;
        let mut root = div()
            .id(self.id.clone())
            .aria_label(self.label.clone())
            .aria_selected(state == CheckState::Checked)
            .flex()
            .items_center()
            .gap(px(8.))
            .cursor_pointer();
        if state == CheckState::Mixed {
            // GPUI has no mixed flag to set, so its value says it to assistive technology.
            root = root.aria_value("mixed");
        }
        if let Some(on_change) = self.on_change {
            root = root.on_click(move |_, window, cx| on_change(state.next(), window, cx));
        }
        let label = self.labelled.then(|| {
            div()
                .id(part(&self.id, "label"))
                .text_size(px(13.))
                .line_height(px(20.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.text)
                .child(self.label.clone())
                .test_support()
        });
        hoverable(root, move |d, hover| {
            crate::seen::note(&hover_id, hover);
            let mark = div()
                .id(mark_id)
                .relative()
                .flex_none()
                .size(px(BOX))
                .rounded(px(4.))
                .border(px(1.5))
                .border_color(mix(mix(p.line2, p.focus_line, hover), p.acc, fill))
                .bg(mix(p.field, p.acc, fill))
                .child(
                    div()
                        .absolute()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .opacity(on * (1.0 - bar))
                        .child(icon(Icon::Check).size(px(10.)).text_color(p.on_acc)),
                )
                .child(
                    div()
                        .absolute()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .opacity(on * bar)
                        .child(div().w(px(8.)).h(px(1.5)).rounded_full().bg(p.on_acc)),
                )
                .test_support();
            d.child(mark).children(label)
        })
    }
}
