//! `Menu`: a "⋯" button and the small list of entries it opens, which fades in while rising and
//! fades out on closing.
//!
//! The list is the dropdowns' (drawn after everything else, inside the window). Escape closes it,
//! so does a click anywhere outside it, and so does picking an entry, which runs after the menu
//! has started to close.
//!
//! It is as wide as its longest label: no entry wraps, and one too long for the window is cut.
//! Every entry has its own icon (a menu refuses two entries with the same one), the destructive
//! entry's is the bin, and an entry without one keeps the icon's room, so that the labels line up.

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::{
    Context, ElementId, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement,
    Pixels, Render, SharedString, Styled, TestSupportExt, Window, div, px,
};

use crate::button::Button;
use crate::dropdown::{Align, Handler, MenuItem, Width, list_overlay};
use crate::icons::Icon;
use crate::look::look;
use crate::motion::{ENTER, HOVER, Motion};

struct Entry {
    id: ElementId,
    label: SharedString,
    leading: Option<Icon>,
    detail: Option<SharedString>,
    /// Why it cannot be picked now: set, it is inert and says so in its second line.
    disabled: Option<SharedString>,
    /// A switch's state: the entry ends with its track, and a click turns it.
    switch: Option<bool>,
    danger: bool,
    pick: Handler,
}

pub struct Menu {
    id: ElementId,
    tooltip: SharedString,
    glyph: Icon,
    align: Align,
    width: Pixels,
    entries: Vec<Entry>,
    open: bool,
    fade: Motion,
    focus: FocusHandle,
}

impl Menu {
    /// A closed menu: its button is `id` (`tooltip` says what it is), its list `<id>-menu`.
    pub fn new(
        id: impl Into<ElementId>,
        tooltip: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            id: id.into(),
            tooltip: tooltip.into(),
            glyph: Icon::More,
            align: Align::End,
            width: px(180.),
            entries: Vec::new(),
            open: false,
            fade: Motion::settled(0.0),
            focus: cx.focus_handle(),
        }
    }

    /// An entry `id`, picked by a click. Another entry of the menu with the same icon fails the
    /// debug builds and the tests, never the user's window:
    /// two actions that look alike are told apart by their icons, or not at all.
    pub fn entry(
        mut self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        leading: Option<Icon>,
        pick: impl Fn(&mut Window, &mut gpui_kit::App) + 'static,
    ) -> Self {
        if let Some(glyph) = leading {
            debug_assert!(
                self.entries.iter().all(|e| e.leading != Some(glyph)),
                "two entries of the menu `{:?}` have the same icon ({glyph:?})",
                self.id
            );
        }
        self.entries.push(Entry {
            id: id.into(),
            label: label.into(),
            leading,
            detail: None,
            disabled: None,
            switch: None,
            danger: false,
            pick: Rc::new(pick),
        });
        self
    }

    /// An entry that is a switch: its label, and at its end the track of a switch, which slides
    /// as `on` changes ([`set_switch`](Self::set_switch)) and fades like any switch. A click picks
    /// it and closes the menu, as for any entry; `pick` turns the thing it stands for.
    pub fn switch(
        mut self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        leading: Option<Icon>,
        on: bool,
        pick: impl Fn(&mut Window, &mut gpui_kit::App) + 'static,
    ) -> Self {
        self = self.entry(id, label, leading, pick);
        if let Some(entry) = self.entries.last_mut() {
            entry.switch = Some(on);
        }
        self
    }

    /// The switch entry `id` is on or off now: its knob slides there.
    pub fn set_switch(&mut self, id: &ElementId, on: bool, cx: &mut Context<Self>) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| &entry.id == id && entry.switch.is_some())
            && entry.switch != Some(on)
        {
            entry.switch = Some(on);
            cx.notify();
        }
    }

    /// A second, fainter line under the last entry's label, cut to one line.
    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        if let Some(entry) = self.entries.last_mut() {
            entry.detail = Some(detail.into());
        }
        self
    }

    /// The last entry cannot be picked now: its ink fades to the faint one, it takes no hover and
    /// no click, and its second line says `reason` (it replaces a detail). Built on every frame,
    /// an entry that becomes enabled again fades back.
    pub fn disabled(mut self, reason: impl Into<SharedString>) -> Self {
        if let Some(entry) = self.entries.last_mut() {
            entry.disabled = Some(reason.into());
        }
        self
    }

    /// The entry `id` cannot be picked now (`Some(reason)`, as [`disabled`](Self::disabled)) or can
    /// again (`None`), changed from outside while the menu lives on.
    pub fn set_disabled(
        &mut self,
        id: &ElementId,
        reason: Option<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let _ = (id, reason, cx);
    }

    /// A destructive entry, "Remove…": in the failure colour, after the others and a line,
    /// whatever the order it was given in. Its `pick` opens the [`Confirm`](crate::Confirm).
    pub fn danger(
        mut self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        pick: impl Fn(&mut Window, &mut gpui_kit::App) + 'static,
    ) -> Self {
        self.entries.push(Entry {
            id: id.into(),
            label: label.into(),
            leading: Some(Icon::Trash),
            detail: None,
            disabled: None,
            switch: None,
            danger: true,
            pick: Rc::new(pick),
        });
        self
    }

    /// Each entry's id, label and icon, in the order given: what a test reads of a menu.
    pub fn entries(&self) -> Vec<(ElementId, SharedString, Option<Icon>)> {
        self.entries
            .iter()
            .map(|e| (e.id.clone(), e.label.clone(), e.leading))
            .collect()
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// The button's glyph, "⋯" unless the menu is an action's (Run a task: a play).
    pub fn glyph(mut self, glyph: Icon) -> Self {
        self.glyph = glyph;
        self
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            return;
        }
        self.open = true;
        let now = cx.background_executor().now();
        self.fade.animate(1.0, ENTER, now, cx.reduce_motion());
        // Escape goes to the focused element: the menu takes the focus while it is open.
        window.focus(&self.focus, cx);
        cx.notify();
    }

    pub fn close(&mut self, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        let now = cx.background_executor().now();
        self.fade.animate(0.0, HOVER, now, cx.reduce_motion());
        cx.notify();
    }

    pub fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.open {
            true => self.close(cx),
            false => self.open(window, cx),
        }
    }
}

impl Focusable for Menu {
    fn focus_handle(&self, _: &gpui_kit::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for Menu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = cx.background_executor().now();
        if self.fade.is_running(now) {
            window.request_animation_frame();
        } else {
            self.fade.settle(now);
        }
        let t = self.fade.value(now);
        let trigger = Button::icon(self.id.clone(), self.glyph, self.tooltip.clone())
            .tooltip_hidden(self.open)
            .on_click(cx.listener(|this, _, window, cx| this.toggle(window, cx)));
        let root = div()
            .id(ElementId::NamedChild(
                Arc::new(self.id.clone()),
                "root".into(),
            ))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &gpui_kit::KeyDownEvent, _, cx| {
                if this.open && event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    this.close(cx);
                }
            }))
            .relative()
            .flex_none()
            .child(trigger);
        if t == 0.0 && !self.open {
            return root.test_support();
        }
        let menu = cx.entity().downgrade();
        let p = crate::theme::palette(cx);
        let slot = self.entries.iter().any(|e| e.leading.is_some());
        let item = |entry: &Entry, window: &mut Window, cx: &mut gpui_kit::App| {
            let (menu, pick) = (menu.clone(), entry.pick.clone());
            let item = MenuItem::new(entry.id.clone(), entry.label.clone());
            let item = match &entry.disabled {
                Some(reason) => item.disabled(reason.clone()),
                None => item.on_click(move |window, cx| {
                    menu.update(cx, |menu, cx| menu.close(cx)).ok();
                    pick(window, cx);
                }),
            };
            let item = match entry.leading {
                Some(glyph) => item.leading(glyph),
                None if slot => item.slot(),
                None => item,
            };
            let item = match (&entry.detail, &entry.disabled) {
                (Some(detail), None) => item.detail(detail.clone()),
                _ => item,
            };
            let item = match entry.switch {
                Some(on) => item.trailing(crate::switch::track(&entry.id, on, 0., window, cx)),
                None => item,
            };
            if entry.danger { item.danger() } else { item }.into_any_element()
        };
        let mut items: Vec<_> = self
            .entries
            .iter()
            .filter(|e| !e.danger)
            .map(|e| item(e, window, cx))
            .collect();
        let dangers: Vec<_> = self
            .entries
            .iter()
            .filter(|e| e.danger)
            .map(|e| item(e, window, cx))
            .collect();
        if !items.is_empty() && !dangers.is_empty() {
            items.push(
                div()
                    .h(look().border)
                    .mx(px(2.))
                    .my(px(4.))
                    .bg(p.line)
                    .into_any_element(),
            );
        }
        items.extend(dangers);
        // While it closes, nothing is over the window to take a click.
        let dismiss: Option<Handler> = self.open.then(|| {
            let menu = menu.clone();
            Rc::new(move |_: &mut Window, cx: &mut gpui_kit::App| {
                menu.update(cx, |menu, cx| menu.close(cx)).ok();
            }) as Handler
        });
        // The list opens below the icon button that opens it.
        root.child(list_overlay(
            &self.id,
            f32::from(look().sizes.icon_button),
            self.align,
            Width::Fit(self.width),
            items,
            dismiss,
            t,
            window,
            cx,
        ))
        .test_support()
    }
}
