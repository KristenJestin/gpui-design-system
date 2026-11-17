//! `FilterChips`: one choice among a few chips that filter a list, each with its label and how
//! many items it would show ("All 35", "Allowed 1"). A click on the chosen chip goes back to the
//! first one, which is the unfiltered list. The chosen chip's fill and ink move over [`PANEL`]; a
//! chip fades its hover over [`HOVER`]. A chip whose count is 0 stays shown, dimmed: the filter
//! is still there to say there is nothing.

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};

use crate::color::mix;
use crate::interact::{fade, selectable};
use crate::look::{Shaped as _, Surface, look};
use crate::motion::HOVER;
use crate::theme::palette;
use crate::typography::Type as _;

type OnChoose = Rc<dyn Fn(usize, &mut Window, &mut App) + 'static>;

/// How much of its ink a chip with nothing to show keeps.
const EMPTY: f32 = 0.5;

/// One chip: its key (its element is `<id>-<key>`), its label and its count.
#[derive(Clone, Debug)]
pub struct FilterChip {
    key: SharedString,
    label: SharedString,
    count: usize,
}

impl FilterChip {
    pub fn new(key: impl Into<SharedString>, label: impl Into<SharedString>, count: usize) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            count,
        }
    }
}

#[derive(IntoElement)]
pub struct FilterChips {
    id: ElementId,
    label: SharedString,
    chips: Vec<FilterChip>,
    chosen: usize,
    on_choose: Option<OnChoose>,
}

impl FilterChips {
    /// The row `id`, labelled `label` for assistive technology (nothing is drawn for it). Each
    /// chip is labelled "<label> <count>". `chosen` is an index of `chips`; the first one is the
    /// unfiltered list.
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        chips: impl IntoIterator<Item = FilterChip>,
        chosen: usize,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            chips: chips.into_iter().collect(),
            chosen,
            on_choose: None,
        }
    }

    /// Called with the index the filter becomes: the chip clicked, or 0 when the chosen one is
    /// clicked again. Not called when the first chip is clicked while it is chosen.
    pub fn on_choose(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_choose = Some(Rc::new(f));
        self
    }
}

fn part(id: &ElementId, name: &str) -> ElementId {
    match id {
        ElementId::Name(text) => ElementId::Name(format!("{text}-{name}").into()),
        other => ElementId::NamedChild(Arc::new(other.clone()), name.to_owned().into()),
    }
}

impl RenderOnce for FilterChips {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let sizes = look().sizes;
        let chips = self
            .chips
            .iter()
            .enumerate()
            .map(|(index, chip)| {
                let id = part(&self.id, &chip.key);
                let is_chosen = index == self.chosen;
                let text = format!("{} {}", chip.label, chip.count);
                let on_choose = self.on_choose.clone();
                // A chip with nothing to show is dimmed over HOVER as its count reaches 0.
                let empty = fade(part(&id, "empty"), chip.count == 0, HOVER, window, cx);
                let chosen = self.chosen;
                let element = div()
                    .id(id.clone())
                    .aria_label(text.clone())
                    .aria_selected(is_chosen)
                    .flex_none()
                    .h(sizes.chip)
                    .px(sizes.chip_padding)
                    .flex()
                    .items_center()
                    .whitespace_nowrap()
                    .control_face()
                    .text_size(px(11.5))
                    .cursor_pointer()
                    .on_click(move |_, window, cx| {
                        let to = if is_chosen { 0 } else { index };
                        if to != chosen
                            && let Some(on_choose) = &on_choose
                        {
                            on_choose(to, window, cx);
                        }
                    });
                // The words go in after the surface, which the hover and the choice paint.
                selectable(element, is_chosen, move |d, hover, on| {
                    crate::seen::note(&id, on);
                    let ink = mix(mix(p.mute, p.text, hover), p.on_acc, on);
                    d.surface(
                        Surface::full()
                            .fill(mix(p.hover.opacity(hover), p.acc, on))
                            .line(mix(p.line2, p.acc, on)),
                    )
                    .text_color(ink)
                    .opacity(1.0 - (1.0 - EMPTY) * empty)
                    .child(text)
                })
            })
            .collect::<Vec<_>>();
        div()
            .id(self.id)
            .aria_label(self.label)
            .flex()
            .flex_wrap()
            .gap(px(5.))
            .children(chips)
            .test_support()
    }
}
