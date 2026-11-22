//! `Tabs`: the row of names that switches a page between its sections, one of them chosen and
//! shown alone. Unlike [`Segmented`] (two or three short words that switch a mode), it takes as
//! many tabs as a page has sections, each with a count when it lists something. The chosen one is
//! marked by a bar under it that slides to the one clicked over [`PANEL`], widening or narrowing
//! to its words, while the inks fade along it; each tab fades its hover. A tab's count scales and
//! fades in when it appears and out when it goes (an empty count is no count), the tab widening and
//! narrowing with it, and its number fades in again when it changes.
//!
//! [`Segmented`]: crate::Segmented

use std::sync::Arc;

use gpui_kit::{
    AnyElement, App, ClickEvent, ElementId, Font, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, TestSupportExt, TextRun, Window,
    div, font, px,
};

use crate::color::mix;
use crate::interact::hoverable;
use crate::look::{Shaped as _, Side, Surface, Which, look};
use crate::motion::{Motion, PANEL, SWAP_FROM};
use crate::reveal::changed;
use crate::theme::palette;
use crate::typography::{MONO, Type, control_font};

/// What a counted tab remembers between frames: the count it last said (it keeps saying it while
/// it leaves) and how present it is, 0 to 1.
struct Counted {
    text: SharedString,
    presence: Motion,
}

type OnChoose = Box<dyn Fn(usize, &mut Window, &mut App) + 'static>;

const TEXT: f32 = 12.5;
const COUNT: f32 = 11.;
/// Between a tab's name and its count, once the count has arrived.
const GAP: f32 = 6.;
const BAR: f32 = 2.;

struct Tab {
    key: SharedString,
    label: SharedString,
    count: Option<SharedString>,
}

#[derive(IntoElement)]
pub struct Tabs {
    id: ElementId,
    label: SharedString,
    tabs: Vec<Tab>,
    chosen: usize,
    on_choose: Option<OnChoose>,
}

impl Tabs {
    /// `label` says what the tabs switch (their accessible name); each tab is `<id>-<key>`, its
    /// count `<id>-<key>-count`, the marker `<id>-marker`.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>, chosen: usize) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            tabs: Vec::new(),
            chosen,
            on_choose: None,
        }
    }

    pub fn tab(mut self, key: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        self.tabs.push(Tab {
            key: key.into(),
            label: label.into(),
            count: None,
        });
        self
    }

    /// A tab whose section lists something: its count follows its name. An empty `count` is no
    /// count: it scales and fades out, and scales and fades in again with the next one.
    pub fn counted(
        mut self,
        key: impl Into<SharedString>,
        label: impl Into<SharedString>,
        count: impl Into<SharedString>,
    ) -> Self {
        self.tabs.push(Tab {
            key: key.into(),
            label: label.into(),
            count: Some(count.into()),
        });
        self
    }

    /// Called with the index of the tab clicked, when it is not the chosen one.
    pub fn on_choose(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_choose = Some(Box::new(f));
        self
    }
}

/// The width of `words` set in `face` at `size`.
fn width_of(words: &SharedString, face: Font, size: f32, window: &Window) -> f32 {
    let run = TextRun {
        len: words.len(),
        font: face,
        color: gpui_kit::Hsla::default(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    f32::from(
        window
            .text_system()
            .shape_line(words.clone(), px(size), &[run], None)
            .width,
    )
}

impl RenderOnce for Tabs {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let sizes = look().sizes;
        let height = f32::from(sizes.tab);
        let pad = f32::from(sizes.tab_padding);
        let slide = window.use_keyed_state(
            ElementId::NamedChild(Arc::new(self.id.clone()), "slide".into()),
            cx,
            |_, _| Motion::settled(self.chosen as f32),
        );
        let now = cx.background_executor().now();
        let target = self.chosen as f32;
        if slide.read(cx).target() != target {
            let reduce = cx.reduce_motion();
            slide.update(cx, |m, _| m.animate(target, PANEL, now, reduce));
        }
        let motion = *slide.read(cx);
        if motion.is_running(now) {
            window.request_animation_frame();
        }
        let at = motion.value(now);
        // A counted tab's count: what it says, how present it is, how wide it is at full size and
        // how far its number has faded back in.
        let reduce = cx.reduce_motion();
        let counts: Vec<Option<Count>> = self
            .tabs
            .iter()
            .map(|tab| {
                let given = tab.count.as_ref()?;
                let target = if given.is_empty() { 0. } else { 1. };
                let state = window.use_keyed_state(
                    ElementId::NamedChild(
                        Arc::new(self.id.clone()),
                        format!("{}-count", tab.key).into(),
                    ),
                    cx,
                    |_, _| Counted {
                        text: given.clone(),
                        presence: Motion::settled(target),
                    },
                );
                let (text, presence) = state.update(cx, |counted, _| {
                    if !given.is_empty() {
                        counted.text = given.clone();
                    }
                    if counted.presence.target() != target {
                        counted.presence.animate(target, PANEL, now, reduce);
                    }
                    (counted.text.clone(), counted.presence)
                });
                if presence.is_running(now) {
                    window.request_animation_frame();
                }
                let presence = presence.value(now);
                (presence > 0.).then(|| {
                    let id =
                        ElementId::Name(format!("{}-{}-count", id_text(&self.id), tab.key).into());
                    Count {
                        width: width_of(&text, font(MONO), COUNT, window),
                        fade: changed(&id, &text, window, cx),
                        id,
                        text,
                        presence,
                    }
                })
            })
            .collect();
        // Each tab is as wide as its words and the part of its count that has arrived: the marker
        // slides from one place and width to the other's.
        let widths: Vec<f32> = self
            .tabs
            .iter()
            .zip(&counts)
            .map(|(tab, count)| {
                let count = count
                    .as_ref()
                    .map_or(0., |count| (GAP + count.width) * count.presence);
                (width_of(&tab.label, control_font(), TEXT, window) + count + 2. * pad).ceil()
            })
            .collect();
        let lefts: Vec<f32> = widths
            .iter()
            .scan(0., |left, width| {
                let at = *left;
                *left += width;
                Some(at)
            })
            .collect();
        let last = widths.len().saturating_sub(1);
        let (low, high) = (
            (at.floor() as usize).min(last),
            (at.ceil() as usize).min(last),
        );
        let part = at - at.floor();
        let lerp = |a: f32, b: f32| a + (b - a) * part;
        let on_choose: Option<Arc<OnChoose>> = self.on_choose.map(Arc::new);
        let chosen = self.chosen;
        let name = |key: &SharedString, tail: Option<&'static str>| {
            let base = format!("{}-{key}", id_text(&self.id));
            match tail {
                Some(tail) => ElementId::Name(format!("{base}-{tail}").into()),
                None => ElementId::Name(base.into()),
            }
        };
        let tabs = self
            .tabs
            .iter()
            .zip(counts)
            .enumerate()
            .map(|(index, (tab, count))| {
                // How near the marker is to this tab: 1 on it, 0 a tab away or more.
                let near = (1.0 - (at - index as f32).abs()).clamp(0.0, 1.0);
                let on_choose = on_choose.clone();
                // The words go in after the fill, which the hover paints.
                let content: Vec<AnyElement> =
                    std::iter::once(tab.label.clone().into_any_element())
                        .chain(count.map(|count| {
                            // It grows from `SWAP_FROM` of its size, and takes its place and its gap as
                            // it does.
                            let scale = SWAP_FROM + (1. - SWAP_FROM) * count.presence;
                            div()
                                .id(count.id)
                                .aria_label(count.text.clone())
                                .flex_none()
                                .flex()
                                .justify_center()
                                .whitespace_nowrap()
                                .ml(px(GAP * count.presence))
                                .w(px(count.width * count.presence))
                                .opacity(count.presence * count.fade)
                                .mono(COUNT * scale)
                                .child(count.text)
                                .test_support()
                                .into_any_element()
                        }))
                        .collect();
                let mut element = div()
                    .id(name(&tab.key, None))
                    .aria_label(tab.label.clone())
                    .aria_selected(index == chosen)
                    .flex_none()
                    .w(px(widths[index]))
                    .h(px(height - BAR))
                    .flex()
                    .items_center()
                    .justify_center()
                    .whitespace_nowrap()
                    .ui_face()
                    .control_face()
                    .text_size(px(TEXT));
                if index != chosen
                    && let Some(on_choose) = on_choose
                {
                    element = element
                        .cursor_pointer()
                        .on_click(move |_: &ClickEvent, window, cx| on_choose(index, window, cx));
                }
                hoverable(element, move |d, hover| {
                    d.surface(
                        Surface::new(px(6.))
                            .at(Which::Top)
                            .fill(p.hover.opacity(hover * (1. - near))),
                    )
                    .text_color(mix(mix(p.faint, p.text, hover), p.text, near))
                    .children(content)
                })
            });
        // The rule under the tabs is the wrapper's border; the tabs scroll sideways when the
        // window is too narrow for all of them, the marker with them.
        div().w_full().edge(Side::Bottom, p.line).child(
            div()
                .id(self.id.clone())
                .aria_label(self.label.clone())
                .relative()
                .w_full()
                .h(px(height))
                .flex()
                .items_start()
                .overflow_x_scroll()
                .children(tabs)
                .child(
                    div()
                        .id(ElementId::Name(
                            format!("{}-marker", id_text(&self.id)).into(),
                        ))
                        .absolute()
                        .bottom_0()
                        .left(px(lerp(lefts[low], lefts[high])))
                        .w(px(lerp(widths[low], widths[high])))
                        .h(px(BAR))
                        .surface(Surface::new(px(2.)).at(Which::Top).fill(p.acc))
                        .test_support(),
                )
                .test_support(),
        )
    }
}

/// A counted tab's count as it is drawn in one frame.
struct Count {
    id: ElementId,
    text: SharedString,
    /// 0 to 1: how far it has scaled in.
    presence: f32,
    /// Its width at full size.
    width: f32,
    /// 1 at rest, lower while a changed number fades back in.
    fade: f32,
}

/// The text of a named id, for the ids of the parts.
fn id_text(id: &ElementId) -> String {
    match id {
        ElementId::Name(name) => name.to_string(),
        other => format!("{other:?}"),
    }
}
