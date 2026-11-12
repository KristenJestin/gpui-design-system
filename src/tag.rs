//! [`Tag`]: the small bordered mono word that follows a title (a state, a kind, a path), drawn
//! once for every place that has one.

use gpui_kit::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, px,
};

use crate::cut::tipped_when_cut;
use crate::reveal::changed;
use crate::theme::palette;
use crate::typography::Type;

/// A tag: its text in a bordered mono chip, the element `<id>`, labelled with its text. A tag is
/// a word or a path someone wrote: it is cut with an ellipsis at 160 px (its whole text in a
/// tooltip) so that the title keeps its room. A tag whose text changes (`On`, `Off`) fades back
/// in rather than jump.
#[derive(IntoElement)]
pub struct Tag {
    id: ElementId,
    text: SharedString,
    shown: f32,
}

impl Tag {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            shown: 1.0,
        }
    }

    /// How much of its opacity it keeps (1 all of it): a row that is fading takes its tag with it.
    pub(crate) fn shown(mut self, shown: f32) -> Self {
        self.shown = shown;
        self
    }
}

impl RenderOnce for Tag {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let fading = changed(&self.id, &self.text, window, cx);
        let element = div()
            .id(self.id)
            .opacity(fading * self.shown)
            .aria_label(self.text.clone())
            .min_w_0()
            .max_w(px(160.))
            .overflow_hidden()
            .mono(11.)
            .text_color(p.faint)
            .px(px(6.))
            .py(px(1.))
            .rounded(px(5.))
            .border_1()
            .border_color(p.line)
            .whitespace_nowrap()
            .text_ellipsis()
            .child(self.text.clone());
        let tag: AnyElement = tipped_when_cut(element, self.text, window, cx);
        tag
    }
}
