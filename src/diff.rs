//! [`DiffLines`]: the lines of a text that changed between two versions, in mono, each marked
//! `+` (added) or `−` (removed) on the palette's `add` and `del` backgrounds, and the lines that
//! stayed with no mark and no fill. The screen says which line is which: nothing here compares
//! two texts.
//!
//! Its parts, for a test: the block `<id>` and each line `<id>-line-<n>`, labelled with its sign
//! and its text (a kept line has a blank sign).

use gpui_kit::{
    ElementId, Hsla, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};

use crate::space;
use crate::theme::{Palette, palette, radius};
use crate::typography::Type;
use crate::unfolding::part;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Added,
    Removed,
    Kept,
}

/// One line of a [`DiffLines`].
#[derive(Clone, Debug)]
pub struct DiffLine {
    kind: Kind,
    text: SharedString,
}

impl DiffLine {
    /// A line the new version has and the old one has not.
    pub fn added(text: impl Into<SharedString>) -> Self {
        Self {
            kind: Kind::Added,
            text: text.into(),
        }
    }

    /// A line the old version had and the new one has not.
    pub fn removed(text: impl Into<SharedString>) -> Self {
        Self {
            kind: Kind::Removed,
            text: text.into(),
        }
    }

    /// A line both versions have.
    pub fn kept(text: impl Into<SharedString>) -> Self {
        Self {
            kind: Kind::Kept,
            text: text.into(),
        }
    }

    /// `+`, `−` (a real minus sign) or a blank.
    pub fn sign(&self) -> &'static str {
        match self.kind {
            Kind::Added => "+",
            Kind::Removed => "−",
            Kind::Kept => " ",
        }
    }

    /// The background behind the line, none for a kept one.
    pub fn fill(&self, p: &Palette) -> Option<Hsla> {
        match self.kind {
            Kind::Added => Some(p.add),
            Kind::Removed => Some(p.del),
            Kind::Kept => None,
        }
    }

    /// The ink of the line and of its sign.
    pub fn ink(&self, p: &Palette) -> Hsla {
        match self.kind {
            Kind::Added => p.ok,
            Kind::Removed => p.fail,
            Kind::Kept => p.mute,
        }
    }
}

#[derive(IntoElement)]
pub struct DiffLines {
    id: ElementId,
    lines: Vec<DiffLine>,
}

impl DiffLines {
    pub fn new(id: impl Into<ElementId>, lines: impl IntoIterator<Item = DiffLine>) -> Self {
        Self {
            id: id.into(),
            lines: lines.into_iter().collect(),
        }
    }
}

impl RenderOnce for DiffLines {
    fn render(self, _: &mut Window, cx: &mut gpui_kit::App) -> impl IntoElement {
        let p = palette(cx);
        let id = self.id;
        div()
            .id(id.clone())
            .w_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(radius::LIST_ROW)
            .border_1()
            .border_color(p.line)
            .children(self.lines.into_iter().enumerate().map(|(at, line)| {
                let label = format!("{} {}", line.sign(), line.text);
                let mut row = div()
                    .id(part(&id, &format!("line-{at}")))
                    .aria_label(label)
                    .w_full()
                    .px(space::SM)
                    .py(px(1.))
                    .flex()
                    .gap(space::SM)
                    .mono(11.5)
                    .text_color(line.ink(&p));
                if let Some(fill) = line.fill(&p) {
                    row = row.bg(fill);
                }
                row.child(div().flex_none().w(px(8.)).child(line.sign()))
                    .child(div().flex_1().min_w_0().child(line.text))
                    .test_support()
            }))
            .test_support()
    }
}
