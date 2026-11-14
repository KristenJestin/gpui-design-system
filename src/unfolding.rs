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

use gpui_kit::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Transformation,
    Window, div, px, radians,
};

use crate::color::mix;
use crate::cut::tipped_when_cut;
use crate::icons::{Icon, icon};
use crate::ids::part;
use crate::interact::hoverable;
use crate::reveal::presence;
use crate::rows::turning;
use crate::seen;
use crate::space;
use crate::theme::{palette, radius};

type OnToggle = Rc<dyn Fn(bool, &mut Window, &mut App) + 'static>;

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

/// One line that stands for a set of rows or groups, and unfolds them under it: a dashed frame
/// with a title, a faint detail (what is in the set) and a chevron at its end. It is the folded
/// form of what it holds, so it unfolds by itself while the screen says
/// [`unfolded_by`](Self::unfolded_by) something (a search found a row in it) and folds back when
/// that stops, unless a click opened it.
///
/// Its parts, for a test: the line `<id>` (it says its hover, 0 to 1), its title `<id>-title`, its
/// detail `<id>-detail`, its chevron `<id>-chevron` and what it holds `<id>-body`.
#[derive(IntoElement)]
pub struct Summary {
    id: ElementId,
    title: SharedString,
    detail: Option<SharedString>,
    open: bool,
    forced: bool,
    on_toggle: Option<OnToggle>,
    body: Vec<AnyElement>,
}

impl Summary {
    /// The row `id` saying `title`, unfolded by a click (`open`) or folded.
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>, open: bool) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            detail: None,
            open,
            forced: false,
            on_toggle: None,
            body: Vec::new(),
        }
    }

    /// What the set holds, in a few words, faint after the title. It is cut with its whole text
    /// at hand.
    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Whether the screen needs what is under the line to be seen now (a search found something
    /// in it): it unfolds, and folds back when this turns false if no click opened it. A click on
    /// the line while it is unfolded by this folds it, until this turns false.
    pub fn unfolded_by(mut self, found: bool) -> Self {
        self.forced = found;
        self
    }

    /// Called with the state a click on the line asks for.
    pub fn on_toggle(mut self, f: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(f));
        self
    }
}

impl ParentElement for Summary {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for Summary {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        // A click on a line that looks open folds it, also while the screen still says something
        // was found in it: the fold is remembered until nothing is found any more.
        let dismissed = window.use_keyed_state(part(&self.id, "dismissed"), cx, |_, _| false);
        if !self.forced && *dismissed.read(cx) {
            dismissed.update(cx, |dismissed, _| *dismissed = false);
        }
        let open = self.open || (self.forced && !*dismissed.read(cx));
        let forced = self.forced;
        let mut line = div()
            .id(self.id.clone())
            .aria_expanded(open)
            .w_full()
            .px(space::MD)
            .py(space::SM)
            .flex()
            .items_center()
            .gap(space::SM)
            .rounded(radius::LIST_ROW)
            .border_1()
            .border_dashed()
            .border_color(p.line2)
            .cursor_pointer()
            .text_size(px(12.))
            .child(
                div()
                    .id(part(&self.id, "title"))
                    .aria_label(self.title.clone())
                    .flex_none()
                    .font_weight(FontWeight::MEDIUM)
                    .child(self.title)
                    .test_support(),
            );
        if let Some(detail) = self.detail {
            let said = div()
                .id(part(&self.id, "detail"))
                .aria_label(detail.clone())
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_color(p.faint)
                .child(detail.clone());
            line = line.child(tipped_when_cut(said, detail, window, cx));
        }
        line =
            line.child(div().flex_1())
                .child(chevron(part(&self.id, "chevron"), open, window, cx));
        let on_toggle = self.on_toggle;
        line = line.on_click(move |_, window, cx| {
            dismissed.update(cx, |dismissed, cx| {
                *dismissed = open && forced;
                cx.notify();
            });
            if let Some(on_toggle) = &on_toggle {
                on_toggle(!open, window, cx);
            }
        });
        let line_id = self.id.clone();
        let line = hoverable(line, move |d, t| {
            seen::note(&line_id, t);
            d.text_color(mix(p.mute, p.text, t)).bg(p.hover.opacity(t))
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
                div().flex().flex_col().pt(space::SM).children(self.body),
            ))
    }
}
