//! `Unfolding`: a row that unfolds its content under it, in place, when its line is clicked.
//!
//! It is controlled, as [`Disclosure`](crate::Disclosure) is: the screen says whether it is `open`
//! and is told when the line is clicked (`on_toggle`). Unlike a disclosure, its line is the
//! screen's own (a name, a sentence, a control), and its content can be a block whose height
//! changes while it is open (lines that wrap): the fold measures the content on every frame it
//! is drawn, so it folds from the height it has now, also inside a scrolling dialog.
//!
//! Its parts, for a test: the line `<id>` (it says its hover, 0 to 1), the chevron `<id>-chevron`
//! (how far it is folded, 0 to 1), the content `<id>-body`.

use std::f32::consts::FRAC_PI_2;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, TestSupportExt, Transformation, Window, div, px, radians,
};

use crate::icons::{Icon, icon};
use crate::interact::hoverable;
use crate::reveal::presence;
use crate::rows::turning;
use crate::seen;
use crate::space;
use crate::theme::{palette, radius};

type OnToggle = Rc<dyn Fn(bool, &mut Window, &mut App) + 'static>;

pub(crate) fn part(id: &ElementId, name: &str) -> ElementId {
    match id {
        ElementId::Name(text) => ElementId::Name(format!("{text}-{name}").into()),
        other => ElementId::NamedChild(Arc::new(other.clone()), name.to_owned().into()),
    }
}

/// A chevron that turns from pointing down (open) to pointing right (folded) over the fold's
/// time, as `id` is open or not.
pub(crate) fn chevron(id: ElementId, open: bool, window: &mut Window, cx: &mut App) -> AnyElement {
    let turned = turning(id.clone(), open, window, cx);
    seen::note(&id, turned);
    div()
        .id(id)
        .flex_none()
        .child(
            icon(Icon::Chevron)
                .size(px(12.))
                .with_transformation(Transformation::rotate(radians(-FRAC_PI_2 * turned))),
        )
        .test_support()
        .into_any_element()
}

/// A line that unfolds its content in place. A click on the line toggles it; what the screen
/// puts in [`leading`](Self::leading) (a checkbox) and [`trailing`](Self::trailing) (a menu) is
/// the control's own: its clicks do not.
#[derive(IntoElement)]
pub struct Unfolding {
    id: ElementId,
    open: bool,
    leading: Option<AnyElement>,
    line: AnyElement,
    trailing: Option<AnyElement>,
    body: AnyElement,
    on_toggle: Option<OnToggle>,
}

impl Unfolding {
    /// The row `id`, `open` or folded.
    pub fn new(id: impl Into<ElementId>, open: bool) -> Self {
        Self {
            id: id.into(),
            open,
            leading: None,
            line: div().into_any_element(),
            trailing: None,
            body: div().into_any_element(),
            on_toggle: None,
        }
    }

    /// A control before the line, after the chevron.
    pub fn leading(mut self, element: impl IntoElement) -> Self {
        self.leading = Some(element.into_any_element());
        self
    }

    /// What the row says: it takes the room the controls leave.
    pub fn line(mut self, element: impl IntoElement) -> Self {
        self.line = element.into_any_element();
        self
    }

    /// A control at the end of the line.
    pub fn trailing(mut self, element: impl IntoElement) -> Self {
        self.trailing = Some(element.into_any_element());
        self
    }

    /// What unfolds under the line.
    pub fn body(mut self, element: impl IntoElement) -> Self {
        self.body = element.into_any_element();
        self
    }

    /// Called with the state a click on the line asks for.
    pub fn on_toggle(mut self, f: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(f));
        self
    }
}

/// A control of a row: its clicks are its own, not the row's.
fn control(id: ElementId, element: AnyElement) -> AnyElement {
    div()
        .id(id)
        .flex_none()
        .on_click(|_, _, cx| cx.stop_propagation())
        .child(element)
        .test_support()
        .into_any_element()
}

impl RenderOnce for Unfolding {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let open = self.open;
        let mut line = div()
            .id(self.id.clone())
            .aria_expanded(open)
            .w_full()
            .min_h(px(30.))
            .px(space::SM)
            .flex()
            .items_center()
            .gap(space::SM)
            .rounded(radius::LIST_ROW)
            .cursor_pointer()
            .text_size(px(12.5))
            .child(chevron(part(&self.id, "chevron"), open, window, cx));
        if let Some(leading) = self.leading {
            line = line.child(control(part(&self.id, "leading"), leading));
        }
        line = line.child(div().flex_1().min_w_0().overflow_hidden().child(self.line));
        if let Some(trailing) = self.trailing {
            line = line.child(control(part(&self.id, "trailing"), trailing));
        }
        if let Some(on_toggle) = self.on_toggle {
            line = line.on_click(move |_, window, cx| on_toggle(!open, window, cx));
        }
        let line_id = self.id.clone();
        let line = hoverable(line, move |d, t| {
            seen::note(&line_id, t);
            d.bg(p.hover.opacity(t))
        });
        div()
            .id(part(&self.id, "root"))
            .w_full()
            .flex()
            .flex_col()
            .child(line)
            .child(presence(
                part(&self.id, "body"),
                open,
                div().pl(px(26.)).pt(px(2.)).pb(space::XS).child(self.body),
            ))
    }
}
