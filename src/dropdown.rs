//! `Dropdown`: a picker's button and the list it opens, and `MenuItem`, one entry of that list.
//!
//! The list is drawn after everything else (`deferred`), on the opaque card colour with a border
//! and a shadow, so that nothing of the page shows through it or takes its clicks. It stays inside
//! the window (`anchored`): it opens upward when there is no room below. While it is open, a
//! click anywhere outside it closes it and goes no further. Opened from a dialog, it is drawn
//! above the dialog ([`layer`](crate::layer)).

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    Anchor, AnyElement, App, BoxShadow, ElementId, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, RenderOnce, SharedString, StatefulInteractiveElement, Styled,
    TestSupportExt, Window, anchored, deferred, div, point, px,
};

use crate::color::mix;
use crate::icons::{Icon, icon};
use crate::interact::{dismiss_on_navigate, hoverable, open_holding_overlay};
use crate::layer;
use crate::look::{Shaped as _, Surface};
use crate::motion::{ENTER, HOVER, Motion, RISE};
use crate::theme::palette;
use crate::typography::Type;

/// The height of a picker's button: the list opens this far below its top.
pub(crate) const TRIGGER: f32 = 28.;
/// The space between the button and its list, on either side.
const GAP: f32 = 4.;

/// Which edge of the button the list lines up with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Start,
    End,
}

pub(crate) type Handler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

/// How wide a list is: a picker's list has the width its owner gave it, a menu's is as wide as
/// its longest row, at least `Fit.0` and at most [`MENU_MAX`] (less in a narrow window), so that
/// a label never wraps and is cut only when it could not fit the window.
#[derive(Clone, Copy)]
pub(crate) enum Width {
    Fixed(Pixels),
    Fit(Pixels),
}

/// The widest a menu grows: a longer label is cut with an ellipsis.
const MENU_MAX: f32 = 420.;

#[derive(IntoElement)]
pub struct Dropdown {
    id: ElementId,
    label: SharedString,
    leading: Option<Icon>,
    strong: bool,
    mono: bool,
    open: bool,
    align: Align,
    width: Pixels,
    items: Vec<AnyElement>,
    on_toggle: Option<Handler>,
}

impl Dropdown {
    /// A picker whose button says `label`; its list is `<id>-menu`, over `<id>-backdrop`.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            leading: None,
            strong: false,
            mono: false,
            open: false,
            align: Align::Start,
            width: px(260.),
            items: Vec::new(),
            on_toggle: None,
        }
    }

    /// An icon before the label.
    pub fn leading(mut self, glyph: Icon) -> Self {
        self.leading = Some(glyph);
        self
    }

    /// The label in the text colour and medium weight: the picker that matters most.
    pub fn strong(mut self) -> Self {
        self.strong = true;
        self
    }

    /// The label in the mono face: a model, an effort.
    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    pub fn item(mut self, item: impl IntoElement) -> Self {
        self.items.push(item.into_any_element());
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = impl IntoElement>) -> Self {
        self.items
            .extend(items.into_iter().map(IntoElement::into_any_element));
        self
    }

    /// Called by a click on the button, and by a click outside the open list: the owner opens
    /// or closes it.
    pub fn on_toggle(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Dropdown {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let toggle = self.on_toggle.clone();
        // The words go in after the fill, which the hover paints.
        let content: Vec<AnyElement> = self
            .leading
            .map(|glyph| {
                icon(glyph)
                    .size(px(14. * glyph.scale()))
                    .text_color(p.faint)
                    .into_any_element()
            })
            .into_iter()
            .chain([
                self.label.into_any_element(),
                icon(Icon::Choose)
                    .size(px(10.))
                    .text_color(p.faint)
                    .into_any_element(),
            ])
            .collect();
        let button = div()
            .id(self.id.clone())
            .h(px(TRIGGER))
            .px(px(9.))
            .flex()
            .items_center()
            .gap(px(6.))
            .text_size(px(12.5))
            .control_face()
            .when(self.strong, |d| d.medium())
            .when(self.mono, |d| d.mono(12.))
            .when_some(toggle, |d, toggle| {
                d.cursor_pointer()
                    .on_click(move |_, window, cx| toggle(window, cx))
            });
        let (open, ink) = (self.open, if self.strong { p.text } else { p.mute });
        let button = hoverable(button, move |d, t| {
            let t = if open { 1.0 } else { t };
            d.surface(Surface::new(px(7.)).fill(p.hover.opacity(t)))
                .text_color(mix(ink, p.text, t))
                .children(content)
        });
        let root = div().relative().flex_none().child(button);
        // The list comes and goes like the "⋯" menu's: it stays drawn until its close ends.
        let fade = window.use_keyed_state(
            ElementId::NamedChild(Arc::new(self.id.clone()), "fade".into()),
            cx,
            |_, _| Motion::settled(0.0),
        );
        let now = cx.background_executor().now();
        let reduce = cx.reduce_motion();
        let (target, duration) = if self.open {
            (1.0, ENTER)
        } else {
            (0.0, HOVER)
        };
        let motion = fade.update(cx, |motion, _| {
            if motion.target() != target {
                motion.animate(target, duration, now, reduce);
            }
            motion.settle(now);
            *motion
        });
        if motion.is_running(now) {
            window.request_animation_frame();
        }
        let t = motion.value(now);
        if t == 0.0 && !self.open {
            return root;
        }
        // While it closes, nothing is over the window to take a click.
        let dismiss = self.open.then_some(self.on_toggle).flatten();
        root.child(list_overlay(
            &self.id,
            TRIGGER,
            self.align,
            Width::Fixed(self.width),
            self.items,
            dismiss,
            t,
            window,
            cx,
        ))
    }
}

/// The list a button opens, drawn after everything else: below the button, or above it when the
/// window has no room below, on the opaque card colour. With a `dismiss`, a backdrop over the
/// whole window takes the click that closes it. `fade` is how far into view it is (1 at rest):
/// an entering list rises into place while it grows opaque.
#[allow(clippy::too_many_arguments)]
pub(crate) fn list_overlay(
    id: &ElementId,
    trigger: f32,
    align: Align,
    width: Width,
    items: Vec<AnyElement>,
    dismiss: Option<Handler>,
    fade: f32,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let p = palette(cx);
    let viewport = window.viewport_size();
    let panel_dismiss = dismiss.clone();
    if let Some(dismiss) = dismiss.clone() {
        open_holding_overlay(
            ElementId::Name(format!("{}-menu", id_text(id)).into()),
            move |window, cx| dismiss(window, cx),
            cx,
        );
    }
    let backdrop = dismiss.map(|dismiss| {
        let on_navigate = dismiss.clone();
        let backdrop = div()
            .id(ElementId::Name(format!("{}-backdrop", id_text(id)).into()))
            .w(viewport.width)
            .h(viewport.height)
            .occlude()
            .on_mouse_down(MouseButton::Left, move |_, window, cx| dismiss(window, cx))
            .test_support();
        anchored()
            .position(point(px(0.), px(0.)))
            .child(dismiss_on_navigate(backdrop, move |window, cx| {
                on_navigate(window, cx)
            }))
    });
    let menu = ElementId::Name(format!("{}-menu", id_text(id)).into());
    let panel = div()
        .id(menu)
        .occlude()
        .when(fade < 1.0, |d| {
            d.relative().top(px(RISE * (1.0 - fade))).opacity(fade)
        })
        .map(|d| match width {
            Width::Fixed(width) => d.w(width),
            Width::Fit(least) => d
                .min_w(least)
                .max_w(px(MENU_MAX).min(viewport.width - px(16.))),
        })
        .p(px(4.))
        .surface(Surface::new(px(10.)).fill(p.card).line(p.line2))
        .drop_shadow(
            vec![BoxShadow {
                color: p.shadow.opacity(0.5),
                offset: point(px(0.), px(12.)),
                blur_radius: px(32.),
                spread_radius: px(0.),
                inset: false,
            }],
            p.shadow,
        )
        .ui_text()
        .text_color(p.text)
        .flex()
        .flex_col()
        .children(items)
        .test_support();
    let panel = match panel_dismiss {
        Some(dismiss) => dismiss_on_navigate(panel, move |window, cx| dismiss(window, cx)),
        None => panel,
    };
    // The padding is the gap on either side.
    let list = anchored()
        .anchor(match align {
            Align::Start => Anchor::TopLeft,
            Align::End => Anchor::TopRight,
        })
        .offset(point(px(0.), px(trigger - GAP)))
        .child(div().py(px(GAP)).child(panel));
    div()
        .absolute()
        .top_0()
        .map(|d| match align {
            Align::Start => d.left_0(),
            Align::End => d.right_0(),
        })
        .child(deferred(div().children(backdrop).child(list)).priority(layer::LIST))
}

fn id_text(id: &ElementId) -> String {
    match id {
        ElementId::Name(name) => name.to_string(),
        other => other.to_string(),
    }
}

/// One entry of a dropdown's list: an icon, its label, a note on the right; picked by a click,
/// or faint and inert without one.
#[derive(IntoElement)]
pub struct MenuItem {
    id: ElementId,
    label: SharedString,
    leading: Option<Icon>,
    note: Option<SharedString>,
    detail: Option<SharedString>,
    slot: bool,
    selected: bool,
    danger: bool,
    on_click: Option<Handler>,
}

impl MenuItem {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            leading: None,
            note: None,
            detail: None,
            slot: false,
            selected: false,
            danger: false,
            on_click: None,
        }
    }

    /// A second, fainter line under the label, cut to one line: what the entry acts on.
    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Keeps the icon's room when the entry has no icon, so that its label starts where the
    /// labels of the entries that have one start.
    pub fn slot(mut self) -> Self {
        self.slot = true;
        self
    }

    /// In the failure colour: the "Remove…" of a "⋯" menu, which opens a confirmation.
    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    pub fn leading(mut self, glyph: Icon) -> Self {
        self.leading = Some(glyph);
        self
    }

    pub fn note(mut self, note: impl Into<SharedString>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// The entry picked now.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn on_click(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for MenuItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let active = self.on_click.is_some();
        let (ink, hover_ink, hover_bg) = match (active, self.danger) {
            (false, _) => (p.faint, p.faint, p.hover.opacity(0.)),
            (true, true) => (p.fail, p.fail, p.fail_bg),
            (true, false) => (p.mute, p.text, p.hover),
        };
        let selected = self.selected;
        let label = self.label;
        let label_id = ElementId::Name(format!("{}-label", id_text(&self.id)).into());
        let detail_id = ElementId::Name(format!("{}-detail", id_text(&self.id)).into());
        // The entry's parts go in after the fill, which the hover paints.
        // The icon has a cell of its own, 14 px wide whatever the glyph's scale (the stop's is
        // larger), so that every label starts at the same x.
        let cell = match (self.leading, self.slot) {
            (None, false) => None,
            (leading, _) => Some(
                div()
                    .w(px(14.))
                    .h(px(14.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .children(leading.map(|glyph| {
                        icon(glyph)
                            .size(px(14. * glyph.scale()))
                            .text_color(p.faint)
                    }))
                    .into_any_element(),
            ),
        };
        // An entry is one line: a label is cut, never wrapped, when it cannot fit.
        let words = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .id(label_id)
                    .aria_label(label.clone())
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(label)
                    .test_support(),
            )
            .when_some(self.detail, |d, detail| {
                d.child(
                    div()
                        .id(detail_id)
                        .aria_label(detail.clone())
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .text_size(px(11.5))
                        .text_color(p.faint)
                        .child(detail)
                        .test_support(),
                )
            })
            .into_any_element();
        let note = self.note.map(|note| {
            div()
                .flex_none()
                .mono(11.)
                .text_color(p.faint)
                .child(note)
                .into_any_element()
        });
        let content: Vec<AnyElement> = cell.into_iter().chain([words]).chain(note).collect();
        let item = div()
            .id(self.id)
            .flex_none()
            .min_h(px(30.))
            .px(px(10.))
            .py(px(5.))
            .flex()
            .items_center()
            .gap(px(8.))
            .text_size(px(13.))
            .when_some(self.on_click, |d, f| {
                d.cursor_pointer()
                    .on_click(move |_, window, cx| f(window, cx))
            });
        hoverable(item, move |d, t| {
            let t = if selected { 1.0 } else { t };
            let fill = if selected { p.sel } else { hover_bg.opacity(t) };
            d.text_color(mix(ink, hover_ink, t))
                .surface(Surface::new(px(6.)).fill(fill))
                .children(content)
        })
    }
}
