//! `Swatches`: a few colours to choose among, each a round button of its own colour. The chosen
//! one has a ring in the ink colour that moves in over [`PANEL`](crate::motion::PANEL) as the
//! choice changes; the others' ring fades in over [`HOVER`](crate::motion::HOVER) under the
//! pointer. Each swatch says its name as its label and in a tooltip. Put it in a
//! [`Field`](crate::Field), which draws the label above it.

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::{
    App, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};

use crate::button::tipped;
use crate::color::mix;
use crate::interact::selectable;
use crate::theme::palette;

type OnChoose = Rc<dyn Fn(usize, &mut Window, &mut App) + 'static>;

/// One colour to choose: its key (the id's end), its name and its colour.
pub struct Swatch {
    key: SharedString,
    title: SharedString,
    colour: Hsla,
}

impl Swatch {
    pub fn new(key: impl Into<SharedString>, title: impl Into<SharedString>, colour: Hsla) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            colour,
        }
    }
}

#[derive(IntoElement)]
pub struct Swatches {
    id: ElementId,
    label: SharedString,
    swatches: Vec<Swatch>,
    chosen: usize,
    on_choose: Option<OnChoose>,
}

impl Swatches {
    /// `label` says what is chosen (its accessible name; a form's [`Field`](crate::Field) draws
    /// it). Each swatch is `<id>-<key>`.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            swatches: Vec::new(),
            chosen: 0,
            on_choose: None,
        }
    }

    pub fn swatch(mut self, swatch: Swatch) -> Self {
        self.swatches.push(swatch);
        self
    }

    pub fn chosen(mut self, index: usize) -> Self {
        self.chosen = index;
        self
    }

    /// Called with the index of the swatch clicked, when it is not the chosen one.
    pub fn on_choose(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_choose = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Swatches {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let id = self.id.clone();
        let swatches = self.swatches.iter().enumerate().map(|(index, swatch)| {
            let chosen = index == self.chosen;
            let colour = swatch.colour;
            let named = |part: &str| match &id {
                ElementId::Name(id) => ElementId::Name(format!("{id}-{}{part}", swatch.key).into()),
                other => ElementId::NamedChild(
                    Arc::new(other.clone()),
                    format!("{}{part}", swatch.key).into(),
                ),
            };
            let mut button = div()
                .id(named(""))
                .aria_label(swatch.title.clone())
                .aria_selected(chosen)
                .flex_none()
                .size(px(28.))
                .p(px(3.))
                .rounded_full()
                .border_2();
            if !chosen && let Some(on_choose) = self.on_choose.clone() {
                button = button
                    .cursor_pointer()
                    .on_click(move |_, window, cx| on_choose(index, window, cx));
            }
            let button = selectable(button, chosen, move |button, hover, chosen| {
                // The ring is the ink's, half way under the pointer, whole once chosen.
                let ring = mix(Hsla::transparent_black(), p.text, chosen.max(hover * 0.5));
                button
                    .border_color(ring)
                    .child(div().size_full().rounded_full().bg(colour))
            });
            tipped_swatch(named("-tip"), button, swatch.title.clone(), window, cx)
        });
        div()
            .id(self.id)
            .aria_label(self.label)
            .flex()
            .flex_wrap()
            .gap(px(8.))
            .children(swatches.collect::<Vec<_>>())
            .test_support()
    }
}

/// A swatch with its name as a tooltip: `selectable` returns a component, which the tooltip
/// wraps in an element of the same size.
fn tipped_swatch(
    id: ElementId,
    button: impl IntoElement,
    title: SharedString,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    tipped(
        div().id(id).flex_none().child(button),
        title,
        28.,
        window,
        cx,
    )
}
