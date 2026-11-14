//! [`BulkBar`]: the bar of the rows selected in a list. It unfolds when the selection is not
//! empty, says how many rows are selected, holds the actions the screen gives and a "Clear", and
//! folds away when the selection empties, still saying what it held.
//!
//! Its parts, for a test: the bar `<id>`, its count `<id>-count` (labelled "2 selected") and its
//! "Clear" `<id>-clear`.

use std::rc::Rc;

use gpui_kit::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};

use crate::button::Button;
use crate::reveal::{changed, presence_of};
use crate::space;
use crate::theme::{palette, radius};
use crate::unfolding::part;

type OnClear = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct BulkBar {
    id: ElementId,
    selected: usize,
    actions: Vec<AnyElement>,
    on_clear: Option<OnClear>,
}

impl BulkBar {
    /// The bar `id` of a selection of `selected` rows: there is no bar at 0.
    pub fn new(id: impl Into<ElementId>, selected: usize) -> Self {
        Self {
            id: id.into(),
            selected,
            actions: Vec::new(),
            on_clear: None,
        }
    }

    /// An action on the selected rows, after the count: a [`Button`], or anything of its size.
    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }

    /// Called when "Clear" is clicked: the screen empties its selection.
    pub fn on_clear(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_clear = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for BulkBar {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let line =
            (self.selected > 0).then(|| SharedString::from(format!("{} selected", self.selected)));
        let bar = Shown {
            id: self.id.clone(),
            count: SharedString::default(),
            actions: self.actions,
            on_clear: self.on_clear,
        };
        presence_of(self.id, line, move |count| {
            Shown { count, ..bar }.into_any_element()
        })
    }
}

/// The bar as it is drawn while it is there, with the count it says.
#[derive(IntoElement)]
struct Shown {
    id: ElementId,
    count: SharedString,
    actions: Vec<AnyElement>,
    on_clear: Option<OnClear>,
}

impl RenderOnce for Shown {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let count_id = part(&self.id, "count");
        let fading = changed(&count_id, &self.count, window, cx);
        let mut clear = Button::quiet(part(&self.id, "clear"), "Clear");
        if let Some(on_clear) = self.on_clear {
            clear = clear.on_click(move |_, window, cx| on_clear(window, cx));
        }
        // The gap under the bar is part of it, so that it folds away with it.
        div().w_full().pb(space::SM).child(
            div()
                .w_full()
                .px(space::SM)
                .py(px(5.))
                .flex()
                .items_center()
                .gap(space::SM)
                .rounded(radius::LIST_ROW)
                .border_1()
                .border_color(p.line)
                .bg(p.card2)
                .text_size(px(12.))
                .text_color(p.mute)
                .child(
                    div()
                        .id(count_id)
                        .aria_label(self.count.clone())
                        .flex_none()
                        .opacity(fading)
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.text)
                        .child(self.count)
                        .test_support(),
                )
                .children(self.actions)
                .child(div().flex_1())
                .child(clear),
        )
    }
}
