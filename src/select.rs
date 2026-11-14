//! `Select`: one option among a list that may grow, more than three, or with long labels (a
//! tracker, a forge, a setup step's kind or place). A 34 px field showing the chosen option; a
//! click opens its list, which fades in while rising, each option with its title and one line,
//! the chosen one with a check that fades along it. Picking one closes the list; so do Escape and
//! a click outside it.
//!
//! A compact select ([`Select::compact`]) is a pill 24 px high that shows the value alone, as a
//! row of a list holds it: its fill and ink follow the option chosen, each option giving its
//! [`Tone`], and move to the next one's over [`PANEL`] when the choice changes. Its list is the
//! same, as wide as its longest option's words, whose sentences it shows.
//!
//! The fields a chosen option needs are the form's: drawn under the select with
//! [`presence`](crate::presence), they unfold when the option is chosen and fold away when another
//! is.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, Bounds, ElementId, FocusHandle, FontWeight, Hsla, InteractiveElement, IntoElement,
    ParentElement, Pixels, RenderOnce, SharedString, StatefulInteractiveElement, Styled,
    TestSupportExt, Window, canvas, div, px,
};

use crate::color::mix;
use crate::dropdown::{Align, Handler, Width, list_overlay};
use crate::icons::{Icon, icon};
use crate::interact::{fade, hoverable, selectable};
use crate::motion::{ENTER, HOVER, Motion, PANEL};
use crate::theme::{Palette, palette};

/// The height of a select's field: the list opens below it.
const HEIGHT: f32 = 34.;
/// The height of a compact select's pill.
const COMPACT: f32 = 24.;
/// The least width of the list of a compact select, whose pill may be narrower than its words.
const COMPACT_LIST: f32 = 220.;

/// How the pill of a compact select is drawn for the option chosen: a fill, an ink and a border,
/// all read from the palette of now so that they follow the theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Tone {
    /// The field's own look: the value says nothing more than itself.
    Outline,
    /// A quiet fill: a value that is the rule.
    Quiet,
    /// What is on, allowed, done.
    Ok,
    /// The strongest fill, the accent: the value that asks for attention.
    Strong,
    /// What waits for the user.
    Wait,
    /// What is refused or failed.
    Fail,
}

impl Tone {
    pub fn fill(self, p: &Palette) -> Hsla {
        match self {
            Self::Outline => p.field,
            Self::Quiet => p.card2,
            Self::Ok => p.ok_bg,
            Self::Strong => p.acc,
            Self::Wait => p.wait_bg,
            Self::Fail => p.fail_bg,
        }
    }

    pub fn ink(self, p: &Palette) -> Hsla {
        match self {
            Self::Outline => p.mute,
            Self::Quiet => p.text,
            Self::Ok => p.ok,
            Self::Strong => p.on_acc,
            Self::Wait => p.wait,
            Self::Fail => p.fail,
        }
    }

    fn line(self, p: &Palette) -> Hsla {
        match self {
            Self::Outline => p.field_line,
            other => other.fill(p),
        }
    }
}

/// The three colours of a pill. Not [`tint`](crate::tint): that one fades whenever its target
/// colour changes, a theme's cross-fade included, which would lag a frame behind the window's own.
/// A pill fades only when its tone changes, so it is keyed on the [`Tone`] and mixes the palette
/// of now.
#[derive(Clone, Copy)]
struct Shades {
    fill: Hsla,
    ink: Hsla,
    line: Hsla,
}

impl Shades {
    fn of(tone: Tone, p: &Palette) -> Self {
        Self {
            fill: tone.fill(p),
            ink: tone.ink(p),
            line: tone.line(p),
        }
    }

    fn toward(self, to: Self, t: f32) -> Self {
        // At rest the colours are exactly the tone's, not a mix that rounds a hair away from it.
        if t >= 1.0 {
            return to;
        }
        Self {
            fill: mix(self.fill, to.fill, t),
            ink: mix(self.ink, to.ink, t),
            line: mix(self.line, to.line, t),
        }
    }
}

type OnChoose = Rc<dyn Fn(usize, &mut Window, &mut App) + 'static>;

/// One option: what it is called and, when its name does not say it, one line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectOption {
    key: SharedString,
    title: SharedString,
    line: Option<SharedString>,
    group: Option<SharedString>,
    tone: Option<Tone>,
}

impl SelectOption {
    /// The option `<select id>-<key>`.
    pub fn new(key: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            line: None,
            group: None,
            tone: None,
        }
    }

    pub fn line(mut self, line: impl Into<SharedString>) -> Self {
        self.line = Some(line.into());
        self
    }

    /// How a compact select's pill is drawn while this option is chosen ([`Tone::Outline`] when
    /// it gives none).
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = Some(tone);
        self
    }

    pub fn key(&self) -> SharedString {
        self.key.clone()
    }

    /// The group it belongs to: the list says the group's name, `<select id>-group-<n>`, above
    /// the first option of each group.
    pub fn group(mut self, group: impl Into<SharedString>) -> Self {
        self.group = Some(group.into());
        self
    }
}

/// What a select remembers between frames: whether its list is open, its fade, its width.
struct Open {
    open: bool,
    fade: Motion,
    width: Rc<Cell<Pixels>>,
    focus: FocusHandle,
    /// A compact select's pill: where its colours came from, the tone they go to and how far.
    from: Shades,
    to: Tone,
    tone: Motion,
}

#[derive(IntoElement)]
pub struct Select {
    id: ElementId,
    options: Vec<SelectOption>,
    chosen: usize,
    fit: bool,
    compact: bool,
    width: Option<Pixels>,
    disabled: bool,
    on_choose: Option<OnChoose>,
}

impl Select {
    /// The field `id`, its list `<id>-menu`, each option `<id>-<key>`; `chosen` is an index of
    /// `options`.
    pub fn new(
        id: impl Into<ElementId>,
        options: impl IntoIterator<Item = SelectOption>,
        chosen: usize,
    ) -> Self {
        Self {
            id: id.into(),
            options: options.into_iter().collect(),
            chosen,
            fit: false,
            compact: false,
            width: None,
            disabled: false,
            on_choose: None,
        }
    }

    /// A pill 24 px high instead of a field 34 px high, for a select inside a row: its width is
    /// its words', or the `width` given, and its list is as wide as its options need. Its fill and
    /// ink follow the chosen option's [`Tone`].
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// As wide as its words (at least 220 px) instead of its column's width.
    pub fn fit(mut self) -> Self {
        self.fit = true;
        self
    }

    /// Exactly `width` wide, its list too: a narrow select beside another.
    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }

    /// Faded, and a click does nothing: turning it on fades it over [`HOVER`] and a list that was
    /// open closes, and it takes no focus. A test reads how far it is enabled with
    /// [`seen`](crate::seen) under `<id>-root`.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called with the index of the option picked, when it is not the chosen one.
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

impl RenderOnce for Select {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let want = self
            .options
            .get(self.chosen)
            .and_then(|option| option.tone)
            .unwrap_or(Tone::Outline);
        let state = window.use_keyed_state(part(&self.id, "open"), cx, |_, cx| Open {
            open: false,
            fade: Motion::settled(0.0),
            width: Rc::new(Cell::new(px(220.))),
            focus: cx.focus_handle(),
            from: Shades::of(want, &p),
            to: want,
            tone: Motion::settled(1.0),
        });
        let now = cx.background_executor().now();
        let reduce = cx.reduce_motion();
        // Enabled or not is a state: it fades over HOVER instead of switching.
        let enabled = fade(part(&self.id, "enabled"), !self.disabled, HOVER, window, cx);
        let (open, motion, width, focus, pill) = state.update(cx, |s, cx| {
            if self.disabled {
                s.open = false;
                // A disabled select takes no focus, and one that had it gives it up.
                if s.focus.is_focused(window) {
                    window.blur(cx);
                }
            }
            let (target, duration) = if s.open { (1.0, ENTER) } else { (0.0, HOVER) };
            if s.fade.target() != target {
                s.fade.animate(target, duration, now, reduce);
            }
            s.fade.settle(now);
            if s.to != want {
                s.from = s.from.toward(Shades::of(s.to, &p), s.tone.value(now));
                s.to = want;
                s.tone.play(0.0, 1.0, Duration::ZERO, PANEL, now, reduce);
            }
            s.tone.settle(now);
            let pill = s.from.toward(Shades::of(s.to, &p), s.tone.value(now));
            (
                s.open,
                s.fade,
                s.width.clone(),
                s.focus.clone(),
                (pill, s.tone),
            )
        });
        let (pill, tone) = pill;
        if motion.is_running(now) || (self.compact && tone.is_running(now)) {
            window.request_animation_frame();
        }
        let t = motion.value(now);
        let set_open = {
            let state = state.clone();
            move |open: bool, window: &mut Window, cx: &mut App| {
                state.update(cx, |s, cx| {
                    s.open = open;
                    cx.notify();
                    if open {
                        window.focus(&s.focus, cx);
                    }
                });
            }
        };
        let chosen = self.options.get(self.chosen).cloned();
        let measured = width.clone();
        let measure = canvas(
            move |bounds: Bounds<Pixels>, _, _| measured.set(bounds.size.width),
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();
        let compact = self.compact;
        let height = if compact { COMPACT } else { HEIGHT };
        // A pill with no width of its own is as wide as its words, which it never cuts.
        let sized = compact && self.width.is_none();
        let mut field = div()
            .id(self.id.clone())
            .aria_label(chosen.as_ref().map(|o| o.title.clone()).unwrap_or_default())
            .aria_expanded(open)
            .relative()
            .h(px(height))
            .px(px(if compact { 9. } else { 10. }))
            .rounded(px(if compact { 7. } else { 8. }))
            .bg(if compact { pill.fill } else { p.field })
            .border_1()
            .flex()
            .items_center()
            .gap(px(if compact { 6. } else { 8. }))
            .text_size(px(if compact { 11.5 } else { 13. }))
            .text_color(if compact { pill.ink } else { p.text })
            .child(measure)
            .child(
                div()
                    .when(!sized, |d| d.flex_1().min_w_0().overflow_hidden())
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .children(chosen.as_ref().map(|o| o.title.clone())),
            )
            .child(if compact {
                icon(Icon::Chevron).size(px(10.)).text_color(pill.ink)
            } else {
                icon(Icon::Choose).size(px(12.)).text_color(p.faint)
            });
        field = match (self.width, self.fit) {
            (Some(width), _) => field.w(width),
            _ if compact => field.flex_none(),
            (None, true) => field.min_w(px(220.)),
            (None, false) => field.w_full(),
        };
        if enabled < 1.0 {
            field = field.opacity(0.35 + 0.65 * enabled);
        }
        if self.disabled {
            // GPUI has no disabled flag to set, so a description says it to assistive technology.
            field = field.aria_description("Disabled");
        } else {
            let toggle = set_open.clone();
            field = field
                .cursor_pointer()
                .on_click(move |_, window, cx| toggle(!open, window, cx));
        }
        let line = if compact { pill.line } else { p.field_line };
        if compact {
            crate::seen::note_colour(&part(&self.id, "pill"), pill.fill);
        }
        let ring_id = part(&self.id, "ring");
        let field = hoverable(field, move |d, hover| {
            let on = if open { 1.0 } else { hover * enabled };
            if compact {
                crate::seen::note(&ring_id, on);
            }
            d.border_color(mix(line, p.focus_line, on))
        });
        let close = set_open.clone();
        let root_id = part(&self.id, "root");
        crate::seen::note(&root_id, enabled);
        let root = div()
            .id(root_id)
            .when(!self.disabled, |d| d.track_focus(&focus))
            .on_key_down(move |event, window, cx| {
                if open && event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    close(false, window, cx);
                }
            })
            .relative()
            .flex()
            .flex_col()
            .map(|d| match (self.width, self.fit) {
                (Some(width), _) => d.flex_none().w(width),
                _ if compact => d.self_start(),
                (None, true) => d.self_start(),
                (None, false) => d.w_full(),
            })
            .child(field);
        if t == 0.0 && !open {
            return root.test_support();
        }
        let on_choose = self.on_choose.clone();
        let mut groups = 0;
        let items = self
            .options
            .iter()
            .enumerate()
            .flat_map(|(index, option)| {
                let header = option
                    .group
                    .clone()
                    .filter(|group| {
                        index == 0 || self.options[index - 1].group.as_ref() != Some(group)
                    })
                    .map(|group| {
                        groups += 1;
                        div()
                            .id(part(&self.id, &format!("group-{groups}")))
                            .aria_label(group.clone())
                            .flex_none()
                            .pt(px(if groups == 1 { 4. } else { 8. }))
                            .pb(px(4.))
                            .px(px(10.))
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.faint)
                            .child(group)
                            .test_support()
                            .into_any_element()
                    });
                let is_chosen = index == self.chosen;
                let (pick, on_choose) = (set_open.clone(), on_choose.clone());
                let mut item = div()
                    .id(part(&self.id, &option.key))
                    .aria_label(option.title.clone())
                    .aria_selected(is_chosen)
                    .relative()
                    .flex_none()
                    .min_h(px(30.))
                    .py(px(6.))
                    .pl(px(30.))
                    .pr(px(10.))
                    .rounded(px(6.))
                    .flex()
                    .flex_col()
                    .justify_center()
                    .cursor_pointer()
                    .on_click(move |_, window, cx| {
                        pick(false, window, cx);
                        if !is_chosen && let Some(on_choose) = &on_choose {
                            on_choose(index, window, cx);
                        }
                    })
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(18.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.text)
                            .child(option.title.clone()),
                    );
                if let Some(line) = &option.line {
                    item = item.child(
                        div()
                            .id(part(&self.id, &format!("{}-line", option.key)))
                            .aria_label(line.clone())
                            .text_size(px(11.5))
                            .line_height(px(16.))
                            .text_color(p.faint)
                            .child(line.clone())
                            .test_support(),
                    );
                }
                let item = selectable(item, is_chosen, move |d, hover, on| {
                    d.bg(p.hover.opacity(hover)).child(
                        div()
                            .absolute()
                            .left(px(10.))
                            .top(px(8.))
                            .opacity(on)
                            .child(icon(Icon::Check).size(px(12.)).text_color(p.text)),
                    )
                })
                .into_any_element();
                header.into_iter().chain([item])
            })
            .collect();
        let dismiss: Option<Handler> = open.then(|| {
            let close = set_open.clone();
            Rc::new(move |window: &mut Window, cx: &mut App| close(false, window, cx)) as Handler
        });
        let list_width = if compact {
            Width::Fit(px(COMPACT_LIST))
        } else {
            Width::Fixed(width.get())
        };
        root.child(list_overlay(
            &self.id,
            height,
            Align::Start,
            list_width,
            items,
            dismiss,
            t,
            window,
            cx,
        ))
        .test_support()
    }
}
