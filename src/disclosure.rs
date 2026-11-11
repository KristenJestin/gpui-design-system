//! `Disclosure`: a quiet line that folds and unfolds what is under it. A short title, a faint
//! summary of what it holds while it is folded, a chevron that turns with the state; the body
//! unfolds with presence under it and folds away when the line is clicked again.
//!
//! It is controlled: the screen says whether it is `open` and is told when the line is clicked
//! (`on_toggle`). Its parts, for a test: the line `<id>`, the chevron `<id>-chevron` (how far it
//! is folded, 0 to 1), the summary `<id>-summary` (how far it is shown, 0 to 1), the body
//! `<id>-body`. The line says its hover (0 to 1) as its value.

use std::f32::consts::FRAC_PI_2;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Transformation, Window, div,
    px, radians,
};

use crate::color::mix;
use crate::icons::{Icon, icon};
use crate::interact::{fade, hoverable};
use crate::motion::HOVER;
use crate::reveal::presence;
use crate::rows::turning;
use crate::seen;
use crate::space;
use crate::theme::palette;

type OnToggle = Rc<dyn Fn(bool, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct Disclosure {
    id: ElementId,
    title: SharedString,
    summary: Option<SharedString>,
    open: bool,
    body: AnyElement,
    on_toggle: Option<OnToggle>,
}

impl Disclosure {
    /// The line `id` saying `title`, `open` or folded.
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>, open: bool) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            summary: None,
            open,
            body: div().into_any_element(),
            on_toggle: None,
        }
    }

    /// One faint line after the title, what the body holds: it fades away while the body is open.
    pub fn summary(mut self, summary: impl Into<SharedString>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// What unfolds under the line.
    pub fn body(mut self, body: impl IntoElement) -> Self {
        self.body = body.into_any_element();
        self
    }

    /// Called with the state a click asks for.
    pub fn on_toggle(mut self, f: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(f));
        self
    }
}

fn part(id: &ElementId, name: &str) -> ElementId {
    match id {
        ElementId::Name(text) => ElementId::Name(format!("{text}-{name}").into()),
        other => ElementId::NamedChild(Arc::new(other.clone()), name.to_owned().into()),
    }
}

impl RenderOnce for Disclosure {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let open = self.open;
        let chevron_id = part(&self.id, "chevron");
        let turned = turning(chevron_id.clone(), open, window, cx);
        let summary_id = part(&self.id, "summary");
        let shown = fade(summary_id.clone(), !open, HOVER, window, cx);
        seen::note(&chevron_id, turned);
        seen::note(&summary_id, shown);
        let chevron = div()
            .id(chevron_id)
            .flex_none()
            .child(
                icon(Icon::Chevron)
                    .size(px(12.))
                    .with_transformation(Transformation::rotate(radians(-FRAC_PI_2 * turned))),
            )
            .test_support();
        let mut line = div()
            .id(self.id.clone())
            .aria_label(self.title.clone())
            .aria_expanded(open)
            .flex()
            .items_center()
            .gap(space::XS)
            .cursor_pointer()
            .text_size(px(12.))
            .child(chevron)
            .child(self.title);
        if let Some(summary) = self.summary {
            line = line.child(
                div()
                    .id(summary_id)
                    .aria_label(summary.clone())
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_color(p.faint)
                    .opacity(shown)
                    .child(summary)
                    .test_support(),
            );
        }
        if let Some(on_toggle) = self.on_toggle {
            line = line.on_click(move |_, window, cx| on_toggle(!open, window, cx));
        }
        let line_id = self.id.clone();
        let line = hoverable(line, move |d, t| {
            seen::note(&line_id, t);
            d.text_color(mix(p.mute, p.text, t))
        });
        div()
            .id(part(&self.id, "root"))
            .flex()
            .flex_col()
            .child(line)
            .child(presence(
                part(&self.id, "body"),
                open,
                div().pt(space::SM).child(self.body),
            ))
    }
}
