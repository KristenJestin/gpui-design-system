//! Toasts: the result of an action the user took (saved, added, removed, started, failed), at the
//! bottom right of the window. Each one is 360 px wide: an icon, one line and an optional second.
//! It rises and fades in, and folds away with its gap. A success leaves after four seconds, held
//! while the pointer is on it; a failure stays until it is dismissed. A notice ([`Toast::notice`])
//! is something told to the user, not a result: it stays eight seconds, can carry several buttons
//! and a click on its lines, and is taken away with [`withdraw`]. Several stack, the newest on top.
//!
//! The shell owns the [`ToastStack`] and draws it last; any view says a result with [`toast`].
//! A result is never a line appended under a form (a scan of the window's sources refuses it); a
//! form's own validation stays under its field ([`Field`](crate::Field)).

use std::rc::Rc;
use std::time::Duration;

use gpui_kit::{
    App, BoxShadow, Context, ElementId, FontWeight, Global, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Task, TestSupportExt,
    WeakEntity, Window, anchored, deferred, div, point, px,
};

use crate::button::{Button, ButtonKind};
use crate::color::mix;
use crate::cut::quoted;
use crate::icons::{Icon, icon};
use crate::interact::hoverable;
use crate::layer;
use crate::listing::{Item, Listing};
use crate::look::{Shaped as _, Surface};
use crate::reveal::changing;
use crate::space;
use crate::theme::palette;
use crate::typography::Type;

/// How long a success stays.
pub const STAYS: Duration = Duration::from_secs(4);
/// How long a notice stays: it says something to read and may ask for an answer.
pub const NOTICE_STAYS: Duration = Duration::from_secs(8);
/// How long a success or a notice stays once the pointer has left it.
pub const STAYS_AFTER_HOVER: Duration = Duration::from_secs(2);

const WIDTH: f32 = 360.;
const EDGE: f32 = 20.;
const GAP: f32 = 8.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Failure,
    /// Something told to the user, not the result of what they did: a notification. It leaves
    /// by itself like a success, after [`NOTICE_STAYS`].
    Notice,
}

impl ToastKind {
    /// How long the toast stays: none for a failure, which stays until it is dismissed.
    fn stays(self) -> Option<Duration> {
        match self {
            ToastKind::Success => Some(STAYS),
            ToastKind::Notice => Some(NOTICE_STAYS),
            ToastKind::Failure => None,
        }
    }
}

/// One result: its kind, its line, an optional second line, its buttons and an optional click on
/// its lines.
#[derive(Clone)]
pub struct Toast {
    kind: ToastKind,
    title: SharedString,
    /// What the title says before the title it quotes: the quote takes one line, whatever
    /// its length.
    quoted: Option<SharedString>,
    detail: Option<SharedString>,
    actions: Vec<Action>,
    open: Option<Run>,
    key: Option<SharedString>,
}

type Run = Rc<dyn Fn(&mut Window, &mut App)>;

/// A button in a toast (`Open`): its click runs `run` and the toast leaves.
#[derive(Clone)]
struct Action {
    label: SharedString,
    kind: ButtonKind,
    run: Run,
}

impl std::fmt::Debug for Toast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Toast")
            .field("kind", &self.kind)
            .field("title", &self.title)
            .field("detail", &self.detail)
            .field(
                "actions",
                &self.actions.iter().map(|a| &a.label).collect::<Vec<_>>(),
            )
            .field("open", &self.open.is_some())
            .finish()
    }
}

impl PartialEq for Toast {
    fn eq(&self, other: &Toast) -> bool {
        self.kind == other.kind
            && self.title == other.title
            && self.detail == other.detail
            && self.key == other.key
            && self.actions.len() == other.actions.len()
            && self
                .actions
                .iter()
                .zip(&other.actions)
                .all(|(a, b)| a.label == b.label && a.kind == b.kind && Rc::ptr_eq(&a.run, &b.run))
            && match (&self.open, &other.open) {
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
    }
}

impl Eq for Toast {}

impl Toast {
    /// Something done: "Task test saved".
    pub fn success(title: impl Into<SharedString>) -> Self {
        Self {
            kind: ToastKind::Success,
            title: title.into(),
            quoted: None,
            detail: None,
            actions: Vec::new(),
            open: None,
            key: None,
        }
    }

    /// Something the user is told, not the result of what they did (something waits for their answer): a
    /// bell where a success has its tick. It leaves after [`NOTICE_STAYS`], held while the
    /// pointer is on it; [`action`](Self::action) buttons and [`on_open`](Self::on_open) let it
    /// be answered from where it is.
    pub fn notice(title: impl Into<SharedString>) -> Self {
        Self {
            kind: ToastKind::Notice,
            ..Self::success(title)
        }
    }

    /// Something that could not be done: "The folder was not removed".
    pub fn failure(title: impl Into<SharedString>) -> Self {
        Self {
            kind: ToastKind::Failure,
            ..Self::success(title)
        }
    }

    /// The line quotes a title after what it says so far ("Detached from "): the title is drawn
    /// on the line's one line, in emphasis, cut with an ellipsis to the room the words leave, and
    /// the toast's [`title`](Self::title) says it whole.
    pub fn quoted_title(mut self, title: &str) -> Self {
        let title = title.lines().next().unwrap_or_default().trim();
        self.quoted = Some(self.title.clone());
        self.title = format!("{}{title}", self.title).into();
        self
    }

    /// The second line: what follows from it, or why it failed.
    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// A button under the lines: its click runs `run`, then the toast leaves. Called again, it
    /// adds a button beside the first. The first is `toast-<n>-action`, the next
    /// `toast-<n>-action-2`, `toast-<n>-action-3`.
    pub fn action(
        self,
        label: impl Into<SharedString>,
        run: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.button(ButtonKind::Secondary, label, run)
    }

    /// As [`Toast::action`], a primary button: the one main answer of a notice (Allow).
    pub fn primary_action(
        self,
        label: impl Into<SharedString>,
        run: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.button(ButtonKind::Primary, label, run)
    }

    /// As [`Toast::action`], a quiet button: the refusal beside the main answer (Deny).
    pub fn quiet_action(
        self,
        label: impl Into<SharedString>,
        run: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.button(ButtonKind::Quiet, label, run)
    }

    fn button(
        mut self,
        kind: ButtonKind,
        label: impl Into<SharedString>,
        run: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.actions.push(Action {
            label: label.into(),
            kind,
            run: Rc::new(run),
        });
        self
    }

    /// The buttons of the toast, in order: their label and their kind.
    pub fn buttons(&self) -> impl Iterator<Item = (&str, ButtonKind)> {
        self.actions.iter().map(|a| (a.label.as_ref(), a.kind))
    }

    /// A click on the lines of the toast, `toast-<n>-open`, runs `run`, then the toast leaves.
    pub fn on_open(mut self, run: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.open = Some(Rc::new(run));
        self
    }

    /// Names what the toast says, so that it is said once: while a toast of the same key is on
    /// screen, pushing another replaces its lines (and its button), stays where it is and stays a
    /// while longer, instead of adding a second one. [`shown`] says whether one is.
    pub fn keyed(mut self, key: impl Into<SharedString>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn kind(&self) -> ToastKind {
        self.kind
    }

    pub fn is_failure(&self) -> bool {
        self.kind == ToastKind::Failure
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn second_line(&self) -> Option<&str> {
        self.detail.as_deref()
    }
}

/// The stack the shell owns, reached by [`toast`].
struct Stack(WeakEntity<ToastStack>);

impl Global for Stack {}

/// Says `toast` in the window's stack. Without a stack (a view tested alone) it is dropped.
pub fn toast(toast: Toast, cx: &mut App) {
    let Some(stack) = cx.try_global::<Stack>().map(|s| s.0.clone()) else {
        return;
    };
    stack.update(cx, |stack, cx| stack.push(toast, cx)).ok();
}

/// Takes the toast with this key off the screen: it folds away with its gap. Nothing when none
/// is shown (it left already).
pub fn withdraw(key: &str, cx: &mut App) {
    let Some(stack) = cx.try_global::<Stack>().map(|s| s.0.clone()) else {
        return;
    };
    stack
        .update(cx, |stack, cx| {
            if let Some(number) = stack
                .live
                .iter()
                .find(|live| live.toast.key.as_deref() == Some(key))
                .map(|live| live.number)
            {
                stack.dismiss(number, cx);
            }
        })
        .ok();
}

/// Whether a toast with this key is on screen.
pub fn shown(key: &str, cx: &App) -> bool {
    let Some(stack) = cx.try_global::<Stack>().and_then(|s| s.0.upgrade()) else {
        return false;
    };
    stack
        .read(cx)
        .live
        .iter()
        .any(|live| live.toast.key.as_deref() == Some(key))
}

struct Live {
    number: usize,
    toast: Toast,
    hovered: bool,
    leaving: Option<Task<()>>,
}

/// The toasts of a window, at its bottom right, above everything else.
pub struct ToastStack {
    live: Vec<Live>,
    shown: Listing<(usize, Toast)>,
    next: usize,
}

impl ToastStack {
    /// The window's stack: [`toast`] says results in it from now on. Its toasts are
    /// `toast-<n>`, numbered from 1, each with its button `toast-<n>-dismiss`.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let this = cx.entity().downgrade();
        cx.set_global(Stack(this));
        Self {
            live: Vec::new(),
            shown: Listing::default(),
            next: 0,
        }
    }

    pub fn push(&mut self, toast: Toast, cx: &mut Context<Self>) {
        // A toast said again replaces the one on screen under its key.
        if let Some(key) = toast.key.clone()
            && let Some(at) = self
                .live
                .iter()
                .position(|live| live.toast.key.as_ref() == Some(&key))
        {
            let number = self.live[at].number;
            let leaving = toast
                .kind
                .stays()
                .map(|stays| self.leave_after(number, stays, cx));
            let live = &mut self.live[at];
            live.toast = toast;
            live.leaving = leaving;
            cx.notify();
            return;
        }
        self.next += 1;
        let number = self.next;
        let leaving = toast
            .kind
            .stays()
            .map(|stays| self.leave_after(number, stays, cx));
        self.live.push(Live {
            number,
            toast,
            hovered: false,
            leaving,
        });
        cx.notify();
    }

    /// The toasts on screen, oldest first.
    pub fn toasts(&self) -> impl Iterator<Item = &Toast> {
        self.live.iter().map(|live| &live.toast)
    }

    pub fn dismiss(&mut self, number: usize, cx: &mut Context<Self>) {
        self.live.retain(|live| live.number != number);
        cx.notify();
    }

    fn leave_after(&self, number: usize, after: Duration, cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(after).await;
            this.update(cx, |this, cx| {
                if this.live.iter().any(|l| l.number == number && !l.hovered) {
                    this.dismiss(number, cx);
                }
            })
            .ok();
        })
    }

    fn hovered(&mut self, number: usize, hovered: bool, cx: &mut Context<Self>) {
        let restart = {
            let Some(live) = self.live.iter_mut().find(|l| l.number == number) else {
                return;
            };
            live.hovered = hovered;
            live.toast.kind.stays().is_some()
        };
        if !restart {
            return;
        }
        let leaving = (!hovered).then(|| self.leave_after(number, STAYS_AFTER_HOVER, cx));
        if let Some(live) = self.live.iter_mut().find(|l| l.number == number) {
            live.leaving = leaving;
        }
    }

    fn card(
        &self,
        number: usize,
        toast: &Toast,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette(cx);
        let failed = toast.kind == ToastKind::Failure;
        let (glyph, ink, wash) = match toast.kind {
            ToastKind::Failure => (Icon::Close, p.fail, p.fail_bg),
            ToastKind::Success => (Icon::Check, p.ok, p.ok_bg),
            ToastKind::Notice => (Icon::Bell, p.text, p.hover),
        };
        let mark = div()
            .mt(px(2.))
            .size(px(15.))
            .flex_none()
            .surface(Surface::full().fill(wash))
            .flex()
            .items_center()
            .justify_center()
            .child(icon(glyph).size(px(9.)).text_color(ink));
        let text = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(match &toast.quoted {
                Some(before) => {
                    let title = toast
                        .title
                        .strip_prefix(before.as_ref())
                        .unwrap_or_default();
                    div()
                        .text_size(px(13.))
                        .line_height(px(19.5))
                        .child(quoted(
                            format!("toast-{number}-title"),
                            before,
                            title,
                            "",
                            window,
                            cx,
                        ))
                        .into_any_element()
                }
                None => changing(
                    ElementId::Name(format!("toast-{number}-title-line").into()),
                    toast.title.clone(),
                    move |title| {
                        div()
                            .id(ElementId::Name(format!("toast-{number}-title").into()))
                            .text_size(px(13.))
                            .line_height(px(19.5))
                            .font_weight(FontWeight::MEDIUM)
                            .child(title)
                            .test_support()
                            .into_any_element()
                    },
                )
                .into_any_element(),
            })
            .children(toast.detail.clone().map(|detail| {
                div()
                    .id(ElementId::Name(format!("toast-{number}-detail").into()))
                    .text_size(px(12.5))
                    .line_height(px(19.))
                    .text_color(p.mute)
                    .child(detail)
                    .test_support()
            }))
            .children((!toast.actions.is_empty()).then(|| {
                let buttons = toast.actions.iter().enumerate().map(|(at, action)| {
                    let (this, run) = (cx.entity().downgrade(), action.run.clone());
                    let id = match at {
                        0 => format!("toast-{number}-action"),
                        at => format!("toast-{number}-action-{}", at + 1),
                    };
                    let id = ElementId::Name(id.into());
                    let button = match action.kind {
                        ButtonKind::Primary => Button::primary(id, action.label.clone()),
                        ButtonKind::Quiet => Button::quiet(id, action.label.clone()),
                        _ => Button::secondary(id, action.label.clone()),
                    };
                    button.on_click(move |_, window, cx| {
                        run(window, cx);
                        this.update(cx, |this, cx| this.dismiss(number, cx)).ok();
                    })
                });
                div()
                    .mt(space::SM)
                    .flex()
                    .flex_wrap()
                    .gap(space::SM)
                    .children(buttons)
            }));
        let dismiss = div()
            .id(ElementId::Name(format!("toast-{number}-dismiss").into()))
            .aria_label("Dismiss")
            .flex_none()
            .size(px(22.))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| this.dismiss(number, cx)));
        let dismiss = hoverable(dismiss, move |d, t| {
            let ink = mix(p.mute, p.text, t);
            d.surface(Surface::new(px(7.)).fill(p.hover.opacity(t)))
                .child(icon(Icon::Close).size(px(12.)).text_color(ink))
        });
        let label = match &toast.detail {
            Some(detail) => format!("{} {detail}", toast.title),
            None => toast.title.to_string(),
        };
        div()
            .id(ElementId::Name(format!("toast-{number}").into()))
            .aria_label(SharedString::from(label))
            .occlude()
            .w(px(WIDTH))
            .flex()
            .items_start()
            .gap(px(10.))
            .pt(px(12.))
            .pb(px(12.))
            .pl(px(14.))
            .pr(px(10.))
            .surface(
                Surface::new(px(12.))
                    .fill(mix(p.card, p.card2, 0.6))
                    .line(if failed { p.fail_line } else { p.line2 }),
            )
            .drop_shadow(
                vec![BoxShadow {
                    color: p.shadow.opacity(0.55),
                    offset: point(px(0.), px(14.)),
                    blur_radius: px(36.),
                    spread_radius: px(0.),
                    inset: false,
                }],
                &p,
            )
            .ui_text()
            .text_color(p.text)
            .on_hover(
                cx.listener(move |this, hovered: &bool, _, cx| this.hovered(number, *hovered, cx)),
            )
            .child(match toast.open.clone() {
                Some(run) => {
                    let this = cx.entity().downgrade();
                    let body = div()
                        .id(ElementId::Name(format!("toast-{number}-open").into()))
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_start()
                        .gap(px(10.))
                        .cursor_pointer()
                        .on_click(move |_, window, cx| {
                            run(window, cx);
                            this.update(cx, |this, cx| this.dismiss(number, cx)).ok();
                        })
                        .child(mark)
                        .child(text);
                    hoverable(body, move |d, t| {
                        d.surface(Surface::new(px(8.)).fill(p.hover.opacity(t * 0.5)))
                    })
                    .into_any_element()
                }
                None => div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_start()
                    .gap(px(10.))
                    .child(mark)
                    .child(text)
                    .into_any_element(),
            })
            .child(dismiss)
            .test_support()
    }
}

impl Render for ToastStack {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = cx.background_executor().now();
        let reduce = cx.reduce_motion();
        // Newest on top: the column is drawn from the newest down to the oldest.
        self.shown.sync(
            self.live
                .iter()
                .rev()
                .map(|live| (live.number.to_string(), (live.number, live.toast.clone()))),
            now,
            reduce,
        );
        if self.shown.rows().next().is_none() {
            return div().into_any_element();
        }
        let rows: Vec<(usize, Toast, crate::listing::Placement)> = self
            .shown
            .rows()
            .map(|row| (row.value.0, row.value.1.clone(), row.placement))
            .collect();
        let cards: Vec<_> = rows
            .into_iter()
            .map(|(number, toast, placement)| {
                Item::new(
                    ElementId::Name(format!("toast-item-{number}").into()),
                    placement,
                    self.card(number, &toast, window, cx),
                )
                .rising()
                .gap(px(GAP))
            })
            .collect();
        let viewport = window.viewport_size();
        deferred(
            anchored().position(point(px(0.), px(0.))).child(
                div()
                    .w(viewport.width)
                    .h(viewport.height)
                    .flex()
                    .flex_col()
                    .justify_end()
                    .items_end()
                    .pr(px(EDGE))
                    .pb(px(EDGE))
                    .child(
                        div()
                            .w(px(WIDTH))
                            .flex()
                            .flex_col()
                            .gap(px(GAP))
                            .children(cards),
                    ),
            ),
        )
        .priority(layer::TOAST)
        .into_any_element()
    }
}
