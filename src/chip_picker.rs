//! `RemovableChip`, a tag that can be taken away, and `ChipPicker`, a multi-select whose value is
//! an ordered list of keys, shown as chips with a search field after them.
//!
//! A chip unfolds sideways and fades in when it is added (its height and the gap around it going
//! with it, so that the row grows and the chips after it slide) and folds away when it is
//! removed. Its fill fades over [`HOVER`] as the pointer enters it, and its remove button is the
//! kit's icon button, which says "Remove <label>" in its tooltip.
//!
//! The picker keeps nothing of its value: the application gives the keys that are chosen and is
//! told, through `on_change`, the list a click or a key asks for. Under the chips, a search field
//! (the caller's `InputState`, as [`SearchField`](crate::SearchField) takes it) opens a menu of
//! what is not chosen yet, filtered by what is typed ([`fuzzy`](crate::fuzzy)), best match first.
//! The menu opens when the field gets the caret or text, below the field or above it when the
//! window has no room, over the dialog it may be in, and stays open after an add so that several
//! can be added in a row.
//!
//! Keys, in the field: the arrows move the mark in the menu, Enter adds the marked item, Escape
//! closes the menu (and nothing around it), Backspace in an empty field removes the last chip.
//!
//! Its parts, for a test: the picker `<id>`, its label `<id>-label`, the count `<id>-count`, the
//! line that says nothing is chosen `<id>-none` (its words `<id>-none-text`), the chips
//! `<id>-chip-<key>` (each with `<id>-chip-<key>-remove`), the field `<id>-search`, the menu
//! `<id>-menu`, its items `<id>-item-<key>` and the line of an empty menu `<id>-empty`.

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::base::input::{InputEvent, InputState};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, Bounds, ClickEvent, Context, ElementId, Entity, Focusable as _, FontWeight,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, ParentElement, Pixels, RenderOnce,
    ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Subscription, TestSupportExt,
    Window, canvas, div, px,
};

use crate::button::Button;
use crate::color::mix;
use crate::dropdown::{Align, Handler, MenuItem, Width, list_overlay};
use crate::fuzzy;
use crate::icons::Icon;
use crate::ids::part;
use crate::interact::{fade, hoverable};
use crate::listing::Listing;
use crate::look::{Shaped as _, Surface, look};
use crate::motion::{ENTER, HOVER, Motion};
use crate::reveal::{Shown, drive, presence};
use crate::search_field::SearchField;
use crate::theme::palette;
use crate::typography::Type as _;

type OnRemove = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;
type OnChange = Rc<dyn Fn(Vec<SharedString>, &mut Window, &mut App) + 'static>;

/// The space after a chip and under it, which comes and goes with it.
const GAP: f32 = 6.;
/// The widest a chip grows: a longer label is cut with an ellipsis.
const CHIP_MAX: f32 = 240.;
/// The tallest the menu grows before it scrolls.
const MENU_MAX: f32 = 272.;

/// A tag that can be taken away: its label, an optional detail in the faint ink and a button that
/// removes it. It is the element `<id>`, labelled with its label and detail.
///
/// A chip first drawn unfolds and fades in over [`PANEL`](crate::motion::PANEL), as it folds away
/// and fades out when it is no longer [`present`](Self::present); the owner builds it on every
/// frame, also while it folds away. [`at_rest`](Self::at_rest) draws it whole from its first
/// frame, for the chips a screen opens with.
#[derive(IntoElement)]
pub struct RemovableChip {
    id: ElementId,
    label: SharedString,
    detail: Option<SharedString>,
    present: bool,
    rest: bool,
    on_remove: Option<OnRemove>,
}

impl RemovableChip {
    /// The chip `id`, its remove button `<id>-remove`, whose tooltip says "Remove <label>".
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            detail: None,
            present: true,
            rest: false,
            on_remove: None,
        }
    }

    /// A second part in the faint ink: what the label stands for.
    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Whether the chip is there: it folds away when this turns false and unfolds when it turns
    /// true again.
    pub fn present(mut self, present: bool) -> Self {
        self.present = present;
        self
    }

    /// Whole from its first frame, with no entrance.
    pub fn at_rest(mut self) -> Self {
        self.rest = true;
        self
    }

    /// Called when the remove button is clicked.
    pub fn on_remove(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_remove = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for RemovableChip {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let rest = self.rest;
        let state = window.use_keyed_state(part(&self.id, "presence"), cx, move |_, _| {
            Shown::new(Motion::settled(if rest { 1.0 } else { 0.0 }))
        });
        let at = drive(&state, self.present, window, cx);
        // Gone, and nothing of it left to draw.
        if !self.present && !at.running {
            return div().hidden().into_any_element();
        }
        let p = palette(cx);
        let look = look();
        // The button keeps its whole height inside the border.
        let height = look.sizes.icon_button + look.border * 2.;
        let accessible: SharedString = match &self.detail {
            Some(detail) => format!("{} {detail}", self.label).into(),
            None => self.label.clone(),
        };
        let words = div()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(6.))
            .child(
                div()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(self.label.clone()),
            )
            .children(self.detail.map(|detail| {
                div()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(px(11.))
                    .text_color(p.faint)
                    .child(detail)
            }));
        let remove = Button::icon(
            part(&self.id, "remove"),
            Icon::Close,
            format!("Remove {}", self.label),
        )
        .when_some(self.on_remove, |button, on_remove| {
            button.on_click(move |event, window, cx| on_remove(event, window, cx))
        });
        let content: Vec<AnyElement> = vec![
            div().min_w_0().flex_1().child(words).into_any_element(),
            div().flex_none().child(remove).into_any_element(),
        ];
        let body = div()
            .id(part(&self.id, "body"))
            .max_w(px(CHIP_MAX))
            .h(height)
            .pl(look.sizes.chip_padding)
            .flex()
            .items_center()
            .gap(px(2.))
            .mono(12.);
        // The fill is one step away from the button's own hover, which it must not hide.
        let body = hoverable(body, move |d, t| {
            d.surface(Surface::new(px(7.)).fill(mix(p.field, p.card, t)).line(mix(
                p.line2,
                p.focus_line,
                t,
            )))
            .text_color(p.text)
            .children(content)
        });
        // The chip's own width, measured whole, is what it unfolds from and folds to.
        let measured = at.width.clone();
        let measure = canvas(
            move |bounds: Bounds<Pixels>, _, _| measured.set(bounds.size.width),
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();
        let whole = div()
            .relative()
            .flex_none()
            .child(body)
            .child(measure)
            .into_any_element();
        let outer = div().id(self.id).aria_label(accessible).flex().flex_none();
        let t = at.t;
        if self.present && !at.running {
            return outer
                .mr(px(GAP))
                .mb(px(GAP))
                .child(whole)
                .test_support()
                .into_any_element();
        }
        // The gap around it unfolds with it: a row that was empty has no gap to jump to.
        outer
            .w(at.width.get() * t)
            .h(height * t)
            .mr(px(GAP * t))
            .mb(px(GAP * t))
            .overflow_hidden()
            .opacity(t)
            .child(whole)
            .test_support()
            .into_any_element()
    }
}

/// One thing the picker offers: its key (what the value holds), its label and an optional detail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChipOption {
    key: SharedString,
    label: SharedString,
    detail: Option<SharedString>,
}

impl ChipOption {
    pub fn new(key: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            detail: None,
        }
    }

    /// A second, fainter line in the menu and a faint part on the chip.
    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// What a picker remembers between frames.
struct Picker {
    open: bool,
    /// The index, among what the menu lists, that the keys are on.
    marked: usize,
    fade: Motion,
    /// The field's width, which the menu takes.
    width: Rc<Cell<Pixels>>,
    scroll: ScrollHandle,
    /// The chips, so that one that left is still drawn while it folds away.
    chips: Listing<(SharedString, Option<SharedString>)>,
    /// The last count above zero, drawn while the count fades out.
    count: usize,
    /// Whether the caret was in the field in the last frame: it opens the menu as it arrives.
    focused: bool,
    _watch: Subscription,
}

impl Picker {
    fn new(input: &Entity<InputState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            open: false,
            marked: 0,
            fade: Motion::settled(0.0),
            width: Rc::new(Cell::new(px(240.))),
            scroll: ScrollHandle::new(),
            chips: Listing::default().at_rest(),
            count: 0,
            focused: false,
            // Any change of the text opens the menu on its first item.
            _watch: cx.subscribe_in(input, window, |picker, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    picker.open = true;
                    picker.marked = 0;
                    cx.notify();
                }
            }),
        }
    }
}

#[derive(IntoElement)]
pub struct ChipPicker {
    id: ElementId,
    label: SharedString,
    input: Entity<InputState>,
    options: Vec<ChipOption>,
    chosen: Vec<SharedString>,
    placeholder: SharedString,
    on_change: Option<OnChange>,
}

impl ChipPicker {
    /// The picker `id`, labelled `label` above it. `input` is the field's text, made and styled
    /// with [`follow_theme`](crate::follow_theme) by the caller; `chosen` is the value, the keys
    /// of `options` in the order they were added.
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        input: &Entity<InputState>,
        options: impl IntoIterator<Item = ChipOption>,
        chosen: impl IntoIterator<Item = SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            input: input.clone(),
            options: options.into_iter().collect(),
            chosen: chosen.into_iter().collect(),
            placeholder: "Add…".into(),
            on_change: None,
        }
    }

    /// What the field says it adds ("Add a need…").
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Called with the whole new list of keys when a chip is removed or an item is added.
    pub fn on_change(
        mut self,
        f: impl Fn(Vec<SharedString>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for ChipPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let id = self.id;
        let state = window.use_keyed_state(part(&id, "state"), cx, |window, cx| {
            Picker::new(&self.input, window, cx)
        });
        let now = cx.background_executor().now();
        let reduce = cx.reduce_motion();
        let n = self.chosen.len();
        let chosen = self.chosen;
        let label_of = |key: &SharedString| {
            self.options
                .iter()
                .find(|option| &option.key == key)
                .map_or((key.clone(), None), |option| {
                    (option.label.clone(), option.detail.clone())
                })
        };

        // The menu: what is not chosen, the best match first.
        let query = self.input.read(cx).value();
        let mut found: Vec<(u32, &ChipOption)> = self
            .options
            .iter()
            .filter(|option| !chosen.contains(&option.key))
            .filter_map(|option| {
                fuzzy::find(
                    &query,
                    &option.label,
                    option.detail.as_deref().unwrap_or_default(),
                )
                .map(|found| (found.score, option))
            })
            .collect();
        found.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
        let found: Vec<&ChipOption> = found.into_iter().map(|(_, option)| option).collect();

        let focused = self.input.focus_handle(cx).contains_focused(window, cx);
        let (open, marked, motion, width, scroll, count, rows) = state.update(cx, |s, _| {
            if focused && !s.focused {
                s.open = true;
                s.marked = 0;
            }
            s.focused = focused;
            s.chips.sync(
                chosen.iter().map(|key| (key.to_string(), label_of(key))),
                now,
                reduce,
            );
            let (target, duration) = if s.open { (1.0, ENTER) } else { (0.0, HOVER) };
            if s.fade.target() != target {
                s.fade.animate(target, duration, now, reduce);
            }
            s.fade.settle(now);
            s.marked = s.marked.min(found.len().saturating_sub(1));
            if n > 0 {
                s.count = n;
            }
            let rows: Vec<_> = s
                .chips
                .rows()
                .map(|row| (row.key.to_owned(), row.value.clone(), row.placement))
                .collect();
            (
                s.open,
                s.marked,
                s.fade,
                s.width.clone(),
                s.scroll.clone(),
                s.count,
                rows,
            )
        });
        if motion.is_running(now) {
            window.request_animation_frame();
        }
        let t = motion.value(now);

        let set_open = {
            let state = state.clone();
            move |open: bool, cx: &mut App| {
                state.update(cx, |s, cx| {
                    if s.open != open {
                        s.open = open;
                        s.marked = 0;
                        cx.notify();
                    }
                });
            }
        };
        // Adds a key, empties the field for the next one and leaves the menu as it is.
        let add = {
            let (input, state, chosen, on_change) = (
                self.input.clone(),
                state.clone(),
                chosen.clone(),
                self.on_change.clone(),
            );
            Rc::new(
                move |key: SharedString, window: &mut Window, cx: &mut App| {
                    let mut next = chosen.clone();
                    if !next.contains(&key) {
                        next.push(key);
                    }
                    input.update(cx, |input, cx| input.clean(window, cx));
                    state.update(cx, |s, cx| {
                        s.marked = 0;
                        cx.notify();
                    });
                    if let Some(on_change) = &on_change {
                        on_change(next, window, cx);
                    }
                },
            )
        };
        let remove = {
            let (chosen, on_change) = (chosen.clone(), self.on_change.clone());
            Rc::new(
                move |key: &SharedString, window: &mut Window, cx: &mut App| {
                    let next: Vec<SharedString> =
                        chosen.iter().filter(|kept| *kept != key).cloned().collect();
                    if let Some(on_change) = &on_change {
                        on_change(next, window, cx);
                    }
                },
            )
        };

        // The label above, and the count after it once something is chosen.
        let shown = fade(part(&id, "count-shown"), n > 0, HOVER, window, cx);
        let count = (shown > 0.0).then(|| {
            div()
                .id(part(&id, "count"))
                .aria_label(count.to_string())
                .opacity(shown)
                .mono(12.)
                .text_color(p.faint)
                .child(count.to_string())
                .test_support()
        });
        let header = div()
            .id(part(&id, "label"))
            .aria_label(self.label.clone())
            .pb(px(8.))
            .flex()
            .items_baseline()
            .gap(px(8.))
            .text_size(px(12.5))
            .line_height(px(18.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(p.text)
            .child(self.label.clone())
            .children(count)
            .test_support();

        // What is chosen, in the order it was added; "None" while nothing is.
        let none = presence(
            part(&id, "none"),
            n == 0,
            div()
                .id(part(&id, "none-text"))
                .aria_label("None")
                .pb(px(8.))
                .text_size(px(12.5))
                .text_color(p.faint)
                .child("None")
                .test_support(),
        );
        let chips = rows.into_iter().map(|(key, (label, detail), placement)| {
            let remove = remove.clone();
            let kept = SharedString::from(key.clone());
            RemovableChip::new(part(&id, &format!("chip-{key}")), label)
                .present(!placement.is_gone())
                .when(placement.is_rest(), |chip| chip.at_rest())
                .when_some(detail, |chip, detail| chip.detail(detail))
                .on_remove(move |_, window, cx| remove(&kept, window, cx))
        });
        let chips = div()
            .id(part(&id, "chips"))
            .flex()
            .flex_wrap()
            .items_start()
            .children(chips)
            .test_support();

        // The field, and the menu that hangs from it.
        let measured = width.clone();
        let measure = canvas(
            move |bounds: Bounds<Pixels>, _, _| measured.set(bounds.size.width),
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();
        let click_open = set_open.clone();
        let mut field = div()
            .relative()
            .w_full()
            .cursor_text()
            // A click on the field opens a menu that Escape closed, the caret being there already.
            .on_mouse_down(MouseButton::Left, move |_, _, cx| click_open(true, cx))
            .child(
                SearchField::new(part(&id, "search"), &self.input, self.label.clone())
                    .placeholder(self.placeholder),
            )
            .child(measure);
        if t > 0.0 || open {
            let focus = self.input.focus_handle(cx);
            let items: Vec<AnyElement> = if found.is_empty() {
                let words = if query.trim().is_empty() {
                    "Nothing left to add"
                } else {
                    "Nothing matches"
                };
                vec![
                    div()
                        .id(part(&id, "empty"))
                        .aria_label(words)
                        .p(px(10.))
                        .text_size(px(12.5))
                        .text_color(p.faint)
                        .child(words)
                        .test_support()
                        .into_any_element(),
                ]
            } else {
                found
                    .iter()
                    .enumerate()
                    .map(|(at, option)| {
                        let (add, key, focus) = (add.clone(), option.key.clone(), focus.clone());
                        MenuItem::new(
                            part(&id, &format!("item-{}", option.key)),
                            option.label.clone(),
                        )
                        .when_some(option.detail.clone(), |item, detail| item.detail(detail))
                        .marked(at == marked)
                        .on_click(move |window, cx| {
                            add(key.clone(), window, cx);
                            window.focus(&focus, cx);
                        })
                        .into_any_element()
                    })
                    .collect()
            };
            let list = div()
                .id(part(&id, "list"))
                .max_h(px(MENU_MAX))
                .overflow_y_scroll()
                .track_scroll(&scroll)
                .flex()
                .flex_col()
                .children(items)
                .into_any_element();
            let dismiss: Option<Handler> = open.then(|| {
                let close = set_open.clone();
                Rc::new(move |_: &mut Window, cx: &mut App| close(false, cx)) as Handler
            });
            field = field.child(list_overlay(
                &id,
                f32::from(look().sizes.field),
                Align::Start,
                Width::Fixed(width.get()),
                vec![list],
                dismiss,
                t,
                window,
                cx,
            ));
        }

        let found_keys: Vec<SharedString> = found.iter().map(|option| option.key.clone()).collect();
        let (input, key_state, last) = (self.input.clone(), state.clone(), chosen.last().cloned());
        let key_add = add.clone();
        let key_remove = remove.clone();
        let close = set_open.clone();
        div()
            .id(id.clone())
            .w_full()
            .flex()
            .flex_col()
            // The menu is closed before anything around the picker is asked: Escape is the menu's
            // while it is open, even with text in the field, which would clear it first.
            .capture_key_down({
                let close = set_open.clone();
                let state = state.clone();
                move |event: &KeyDownEvent, _, cx| {
                    if event.keystroke.key == "escape" && state.read(cx).open {
                        cx.stop_propagation();
                        close(false, cx);
                    }
                }
            })
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                let (open, marked) = {
                    let s = key_state.read(cx);
                    (s.open, s.marked)
                };
                match event.keystroke.key.as_str() {
                    "down" | "up" if !open => {
                        cx.stop_propagation();
                        close(true, cx);
                    }
                    key @ ("down" | "up") => {
                        cx.stop_propagation();
                        let last_item = found_keys.len().saturating_sub(1);
                        key_state.update(cx, |s, cx| {
                            s.marked = if key == "down" {
                                (marked + 1).min(last_item)
                            } else {
                                marked.saturating_sub(1)
                            };
                            s.scroll.scroll_to_item(s.marked);
                            cx.notify();
                        });
                    }
                    "enter" => {
                        if !open {
                            cx.stop_propagation();
                            close(true, cx);
                        } else if let Some(key) = found_keys.get(marked) {
                            cx.stop_propagation();
                            key_add(key.clone(), window, cx);
                        }
                    }
                    "backspace" if input.read(cx).value().is_empty() => {
                        if let Some(last) = &last {
                            cx.stop_propagation();
                            key_remove(last, window, cx);
                        }
                    }
                    "tab" => close(false, cx),
                    _ => {}
                }
            })
            .child(header)
            .child(none)
            .child(chips)
            .child(field)
            .test_support()
    }
}
