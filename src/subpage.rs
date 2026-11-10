//! `Subpage`: a list of rows that opens the page of one of them in place of itself, inside the
//! tab that holds it (the Features tabs). The page comes in from the right, fading in, while the
//! list leaves to the left; going back is the reverse. Above the page a path line,
//! `Features / Review`, whose first word goes back, as Escape does.
//!
//! An entry can be faded (`faded`): a feature that is off where it is turned on is still in its
//! place in the list, at reduced emphasis, its line replaced by a link the application handles
//! (`on_link`) and that opens nothing of the list. The fade and the change of line move over
//! `PANEL`.
//!
//! The component holds no choice: the screen says which entry is open (`open`) and is told when
//! the user opens one or goes back (`on_open`). A screen that keeps its choice keeps the page
//! open while another tab is shown, and the page is simply there when the tab comes back.

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyView, App, ClickEvent, ElementId, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, TestSupportExt,
    Window, div, px,
};

use crate::color::mix;
use crate::icons::{Icon, icon};
use crate::interact::{fade, hoverable};
use crate::motion::{Motion, PANEL};
use crate::rows::{ListCard, ListRow};
use crate::theme::{Palette, palette};
use crate::typography::Type;

type OnOpen = Rc<dyn Fn(Option<SharedString>, &mut Window, &mut App) + 'static>;
type OnLink = Rc<dyn Fn(SharedString, &mut Window, &mut App) + 'static>;
type OnClick = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// How far a page travels while it comes in and the list goes out, in pixels: they move a little
/// and fade, as the detail drawer does, rather than cross the whole width.
const SLIDE: f32 = 32.;

/// How much of its opacity a faded row keeps.
const FADED: f32 = 0.6;

struct Entry {
    key: SharedString,
    icon: Icon,
    title: SharedString,
    tag: SharedString,
    line: SharedString,
}

/// What a subpage remembers between frames: how far the page is in (0 the list, 1 the page), the
/// entry it last showed (whose row keeps its mark while the list comes back, and whose name the
/// path line keeps while it leaves), the page itself while it slides out, and whether Escape has been given its focus.
struct Slide {
    motion: Motion,
    last: Option<SharedString>,
    page: Option<AnyView>,
    focused: bool,
}

#[derive(IntoElement)]
pub struct Subpage {
    id: ElementId,
    label: SharedString,
    entries: Vec<Entry>,
    open: Option<SharedString>,
    on_open: Option<OnOpen>,
    /// The entries that are faded, each with the words of its link.
    faded: Vec<(SharedString, SharedString)>,
    on_link: Option<OnLink>,
    page: Option<AnyView>,
}

impl Subpage {
    /// `label` names the list and starts the path line (`Features`). The list is `<id>-list`, an
    /// entry `<id>-<key>` (its tag `<id>-<key>-tag-0`), the path line `<id>-path` and the word
    /// that goes back `<id>-back`, the page's frame `<id>-page`.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            entries: Vec::new(),
            open: None,
            on_open: None,
            faded: Vec::new(),
            on_link: None,
            page: None,
        }
    }

    /// A row: its icon, its name, its state said once as `tag` (`On`, `Off`, `2 on`) and the one
    /// `line` that says what it is for, never its state.
    pub fn entry(
        mut self,
        key: impl Into<SharedString>,
        icon: Icon,
        title: impl Into<SharedString>,
        tag: impl Into<SharedString>,
        line: impl Into<SharedString>,
    ) -> Self {
        self.entries.push(Entry {
            key: key.into(),
            icon,
            title: title.into(),
            tag: tag.into(),
            line: line.into(),
        });
        self
    }

    /// Fades the entry `key` for as long as this is said: it keeps its place, shows `link` (`Off in
    /// Settings`) in place of its line and its tag, and opens no page. An entry that stops being
    /// faded gets its line and its tag back. The link is `<id>-<key>-off`.
    pub fn faded(mut self, key: impl Into<SharedString>, link: impl Into<SharedString>) -> Self {
        self.faded.push((key.into(), link.into()));
        self
    }

    /// Called with the key of the faded entry whose link was clicked.
    pub fn on_link(mut self, f: impl Fn(SharedString, &mut Window, &mut App) + 'static) -> Self {
        self.on_link = Some(Rc::new(f));
        self
    }

    /// The key of the entry whose page is shown, if any.
    pub fn open(mut self, key: Option<&str>) -> Self {
        self.open = key.map(|key| SharedString::from(key.to_string()));
        self
    }

    /// Called with the key of the entry clicked, or `None` when the path line or Escape goes back.
    pub fn on_open(
        mut self,
        f: impl Fn(Option<SharedString>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(f));
        self
    }

    /// The page of the entry that is open, a view: the subpage keeps it until the page has slid
    /// out, so the screen gives it only while an entry is open and nothing while it leaves.
    pub fn page(mut self, page: impl Into<AnyView>) -> Self {
        self.page = Some(page.into());
        self
    }
}

impl RenderOnce for Subpage {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let id_text = match &self.id {
            ElementId::Name(name) => name.to_string(),
            other => format!("{other:?}"),
        };
        let part = |tail: &str| ElementId::Name(format!("{id_text}-{tail}").into());
        let key = ElementId::NamedChild(Arc::new(self.id.clone()), "slide".into());
        // A faded entry opens nothing.
        let opened = self
            .open
            .clone()
            .filter(|key| !self.faded.iter().any(|(faded, _)| faded == key));
        let open = opened.is_some();
        let state = window.use_keyed_state(key, cx, |_, _| Slide {
            motion: Motion::settled(if open { 1.0 } else { 0.0 }),
            last: opened.clone(),
            page: None,
            focused: false,
        });
        let focus = window
            .use_keyed_state(
                ElementId::NamedChild(Arc::new(self.id.clone()), "focus".into()),
                cx,
                |_, cx| cx.focus_handle(),
            )
            .read(cx)
            .clone();

        let now = cx.background_executor().now();
        let reduce = cx.reduce_motion();
        let target = if open { 1.0 } else { 0.0 };
        let (motion, last, focused, page) = state.update(cx, |slide, _| {
            if opened.is_some() {
                slide.last = opened.clone();
                slide.page = self.page.clone();
            }
            if slide.motion.target() != target {
                slide.motion.animate(target, PANEL, now, reduce);
            }
            (
                slide.motion,
                slide.last.clone(),
                slide.focused,
                slide.page.clone(),
            )
        });
        let running = motion.is_running(now);
        if running {
            window.request_animation_frame();
        } else {
            state.update(cx, |slide, _| {
                slide.motion.settle(now);
                if !open {
                    slide.page = None;
                }
            });
        }
        // Escape reaches the focused element: the page takes the focus as it opens, unless
        // something in it (a field) already holds it.
        if open != focused {
            state.update(cx, |slide, _| slide.focused = open);
            if open {
                let focus = focus.clone();
                window.defer(cx, move |window, cx| {
                    if !focus.contains_focused(window, cx) {
                        window.focus(&focus, cx);
                    }
                });
            }
        }
        let at = motion.value(now);

        let on_open = self.on_open;
        let on_link = self.on_link;
        let list_shown = at < 1.0;
        let page_shown = at > 0.0 || open;
        let title_of = |key: &SharedString| {
            self.entries
                .iter()
                .find(|entry| &entry.key == key)
                .map_or_else(|| key.clone(), |entry| entry.title.clone())
        };

        let list = list_shown.then(|| {
            let rows = self.entries.iter().map(|entry| {
                let id = part(&entry.key);
                let link = self
                    .faded
                    .iter()
                    .find(|(key, _)| key == &entry.key)
                    .map(|(_, link)| link.clone());
                let off = fade(
                    part(&format!("{}-faded", entry.key)),
                    link.is_some(),
                    PANEL,
                    window,
                    cx,
                );
                let kept = kept_link(
                    part(&format!("{}-kept", entry.key)),
                    link.clone(),
                    window,
                    cx,
                );
                let mut row = ListRow::new(id.clone(), entry.title.clone())
                    .leading(icon(entry.icon).text_color(mix(p.mute, p.faint, off)))
                    // The row that was left keeps its mark while the list comes back, then lets go.
                    .marked(at > 0.0 && last.as_ref() == Some(&entry.key));
                match kept {
                    None => {
                        row = row.tag(entry.tag.clone()).line(entry.line.clone());
                    }
                    Some(words) => {
                        // The link answers only while the entry is faded, not while it fades out.
                        let on_click = link.and(on_link.clone()).map(|on_link| {
                            let key = entry.key.clone();
                            Box::new(move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                                on_link(key.clone(), window, cx)
                            }) as OnClick
                        });
                        row = row
                            .tags_shown(1.0 - off)
                            .below(faded_line(
                                (part(&format!("{}-line", entry.key)), entry.line.clone()),
                                (part(&format!("{}-off", entry.key)), words),
                                on_click,
                                off,
                                p,
                            ))
                            .decorate(move |d| {
                                d.opacity(1.0 - (1.0 - FADED) * off)
                                    .text_color(mix(p.text, p.faint, off))
                            });
                        if off < 1.0 {
                            row = row.tag(entry.tag.clone());
                        }
                    }
                }
                match on_open
                    .clone()
                    .filter(|_| !self.faded.iter().any(|(key, _)| key == &entry.key))
                {
                    Some(on_open) => {
                        let key = entry.key.clone();
                        row.on_click(move |_, window, cx| on_open(Some(key.clone()), window, cx))
                    }
                    None => row,
                }
            });
            let card = ListCard::new(part("list")).rows(rows);
            div()
                .w_full()
                .relative()
                .when(open, |d| d.absolute().top_0().left_0())
                .left(px(-SLIDE * at))
                .opacity(1.0 - at)
                .child(card)
        });

        let page = page_shown.then(|| {
            let mut back = div()
                .id(part("back"))
                .aria_label(self.label.clone())
                .cursor_pointer()
                .child(self.label.clone());
            if let Some(on_open) = on_open.clone().filter(|_| open) {
                back = back.on_click(move |_, window, cx| on_open(None, window, cx));
            }
            let back = hoverable(back, move |d, hover| {
                d.text_color(mix(p.faint, p.text, hover))
            });
            let path = div()
                .id(part("path"))
                .aria_label(match &last {
                    Some(key) => format!("{} / {}", self.label, title_of(key)),
                    None => self.label.to_string(),
                })
                .flex()
                .items_baseline()
                .gap(px(6.))
                .mono(12.)
                .text_color(p.faint)
                .whitespace_nowrap()
                .child(back)
                .child("/")
                .child(
                    div()
                        .text_color(p.text)
                        .children(last.as_ref().map(title_of)),
                )
                .test_support();
            let escape = on_open.clone();
            div()
                .id(part("page"))
                .track_focus(&focus)
                .w_full()
                .flex()
                .flex_col()
                .gap(px(16.))
                .relative()
                .when(!open, |d| d.absolute().top_0().left_0())
                .left(px(SLIDE * (1.0 - at)))
                .opacity(at)
                // How far the page is in, for a test and for assistive technology.
                .aria_value(format!("{at}"))
                .when(open, |d| {
                    d.on_key_down(move |event: &KeyDownEvent, window, cx| {
                        if event.keystroke.key == "escape"
                            && let Some(on_open) = &escape
                        {
                            cx.stop_propagation();
                            on_open(None, window, cx);
                        }
                    })
                })
                .child(path)
                .children(page)
                .test_support()
        });

        div()
            .id(self.id.clone())
            .w_full()
            .relative()
            // Part way, the one that leaves and the one that arrives are both drawn: neither may
            // spill out of the tab.
            .when(running, |d| d.overflow_hidden())
            .children(page)
            .children(list)
            .test_support()
    }
}

/// The words a faded entry last said, kept so that its link can be drawn while it fades out: the
/// ones it says now, else the ones it said before (`None` for an entry that never was faded).
fn kept_link(
    id: ElementId,
    now: Option<SharedString>,
    window: &mut Window,
    cx: &mut App,
) -> Option<SharedString> {
    let state = window.use_keyed_state(id, cx, |_, _| None::<SharedString>);
    state.update(cx, |kept, _| {
        if now.is_some() {
            kept.clone_from(&now);
        }
        kept.clone()
    })
}

/// The line of an entry that is or was faded, `off` of the way to its link (0 the line, 1 the
/// link): the one that is not there is drawn over the other while they cross, and alone once it is
/// all the way.
fn faded_line(
    (line_id, line): (ElementId, SharedString),
    (link_id, words): (ElementId, SharedString),
    on_click: Option<OnClick>,
    off: f32,
    p: Palette,
) -> impl IntoElement {
    let clickable = on_click.is_some();
    let link = div()
        .id(link_id)
        .aria_label(words.clone())
        // How far the entry is faded, for a test and for assistive technology.
        .aria_value(format!("{off}"))
        .when(clickable, |d| d.cursor_pointer())
        .child(words);
    let link = hoverable(
        match on_click {
            Some(on_click) => link.on_click(on_click),
            None => link,
        },
        move |d, hover| d.text_color(mix(p.faint, p.text, hover)),
    );
    div()
        .relative()
        .text_size(px(12.5))
        .when(off < 1.0, |d| {
            d.child(
                div()
                    .id(line_id)
                    .aria_label(line.clone())
                    .text_color(p.mute)
                    .opacity(1.0 - off)
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(line)
                    .test_support(),
            )
        })
        .when(off > 0.0, |d| {
            d.child(
                div()
                    .when(off < 1.0, |d| d.absolute().top_0().left_0())
                    .opacity(off)
                    .child(link),
            )
        })
}
