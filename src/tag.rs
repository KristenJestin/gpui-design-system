//! [`Tag`]: the small bordered mono word that follows a title (a state, a kind, a path), drawn
//! once for every place that has one.

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::{
    AnyElement, App, Bounds, ElementId, InteractiveElement, IntoElement, ParentElement, Pixels,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, canvas, div, px,
};

use crate::cut::tipped_when_cut;
use crate::look::{Shaped as _, Surface, look};
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
    /// The room before it, in pixels.
    before: f32,
    /// Where it says how wide it is drawn, border included.
    measured: Option<Rc<Cell<Pixels>>>,
}

impl Tag {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            shown: 1.0,
            before: 0.0,
            measured: None,
        }
    }

    /// The room before it, which a row that has it unfolds and folds away with it.
    pub(crate) fn before(mut self, before: f32) -> Self {
        self.before = before;
        self
    }

    /// Says its drawn width in `measured` on each frame, for a row that folds it away from there.
    pub(crate) fn measured(mut self, measured: Rc<Cell<Pixels>>) -> Self {
        self.measured = Some(measured);
        self
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
        let border = look().border;
        let measure = self.measured.map(|measured| {
            // The border box: the tag's border is part of its width, which a padding box leaves out.
            canvas(
                move |bounds: Bounds<Pixels>, _, _| measured.set(bounds.size.width + border * 2.),
                |_, _, _, _| {},
            )
            .absolute()
            .size_full()
        });
        let element = div()
            .id(self.id)
            .relative()
            .ml(px(self.before))
            .opacity(fading * self.shown)
            .aria_label(self.text.clone())
            .min_w_0()
            .max_w(px(160.))
            .overflow_hidden()
            .mono(11.)
            .text_color(p.faint)
            .px(px(6.))
            .py(px(1.))
            .surface(Surface::new(px(5.)).line(p.line))
            .whitespace_nowrap()
            .text_ellipsis()
            .children(measure)
            .child(self.text.clone());
        let tag: AnyElement = tipped_when_cut(element, self.text, window, cx);
        tag
    }
}
