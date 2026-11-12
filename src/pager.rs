//! One page of several at a time. `sliding` draws the page shown and, when the index changes,
//! slides it in from the side of the move (the next from the right, the previous from the left)
//! over [`PANEL`] while it fades in, inside its frame, which holds still and clips it. `Dots`
//! says it, a dot per page: the page shown is a longer dot, a page done is filled with the
//! primary; each change fades.
//!
//! For a test: the frame of `sliding` notes how far it is in (0 to 1) under its id; each dot is
//! `<id>-<n>` (from 0), labelled `n of m` (`, done` once done) and selected while its page is
//! shown, and notes how far it is done under `<id>-<n>-done`.

use std::sync::Arc;

use gpui_kit::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};

use crate::color::mix;
use crate::interact::fade;
use crate::motion::{HOVER, Motion, PANEL};
use crate::seen;
use crate::space;
use crate::theme::palette;

/// How far a page comes from.
const SLIDE: f32 = 24.0;

/// `content`, page `index` of several: when `index` differs from the last frame's, it slides in
/// from the side of the move. Build it on every frame; `id` keeps the page last shown.
pub fn sliding(id: impl Into<ElementId>, index: usize, content: impl IntoElement) -> Sliding {
    Sliding {
        id: id.into(),
        index,
        content: content.into_any_element(),
    }
}

#[derive(IntoElement)]
pub struct Sliding {
    id: ElementId,
    index: usize,
    content: AnyElement,
}

struct Paged {
    index: usize,
    /// 1 coming from the right, -1 from the left.
    side: f32,
    motion: Motion,
}

impl RenderOnce for Sliding {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let key = ElementId::NamedChild(Arc::new(self.id.clone()), "sliding".into());
        let index = self.index;
        let state = window.use_keyed_state(key, cx, move |_, _| Paged {
            index,
            side: 1.0,
            motion: Motion::settled(1.0),
        });
        let now = cx.background_executor().now();
        let reduce = cx.reduce_motion();
        let (t, side, running) = state.update(cx, |paged, _| {
            if paged.index != index {
                paged.side = if index > paged.index { 1.0 } else { -1.0 };
                paged.index = index;
                paged
                    .motion
                    .play(0.0, 1.0, std::time::Duration::ZERO, PANEL, now, reduce);
            }
            (
                paged.motion.value(now),
                paged.side,
                paged.motion.is_running(now),
            )
        });
        if running {
            window.request_animation_frame();
        }
        seen::note(&self.id, t);
        // The frame holds still and the page moves in it: nothing is drawn past its edges.
        div()
            .id(self.id)
            .w_full()
            .overflow_hidden()
            .test_support()
            .child(
                div()
                    .w_full()
                    .relative()
                    .left(px(side * SLIDE * (1.0 - t)))
                    .opacity(t)
                    .child(self.content),
            )
    }
}

/// A dot per page: which is shown, which are done.
#[derive(IntoElement)]
pub struct Dots {
    id: ElementId,
    label: SharedString,
    current: usize,
    done: Vec<bool>,
}

impl Dots {
    /// The dots `id`, named `label` for who reads the screen.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            current: 0,
            done: Vec::new(),
        }
    }

    /// The page shown, from 0.
    pub fn current(mut self, current: usize) -> Self {
        self.current = current;
        self
    }

    /// One more page, done or not.
    pub fn dot(mut self, done: bool) -> Self {
        self.done.push(done);
        self
    }
}

fn part(id: &ElementId, name: &str) -> ElementId {
    match id {
        ElementId::Name(text) => ElementId::Name(format!("{text}-{name}").into()),
        other => ElementId::NamedChild(Arc::new(other.clone()), name.to_owned().into()),
    }
}

impl RenderOnce for Dots {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let count = self.done.len();
        let dots: Vec<_> = self
            .done
            .iter()
            .enumerate()
            .map(|(n, &done)| {
                let shown = n == self.current;
                let done_id = part(&self.id, &format!("{n}-done"));
                let filled = fade(done_id.clone(), done, HOVER, window, cx);
                let long = fade(
                    part(&self.id, &format!("{n}-current")),
                    shown,
                    PANEL,
                    window,
                    cx,
                );
                seen::note(&done_id, filled);
                let said = match done {
                    true => format!("{} of {count}, done", n + 1),
                    false => format!("{} of {count}", n + 1),
                };
                div()
                    .id(part(&self.id, &n.to_string()))
                    .aria_label(SharedString::from(said))
                    .aria_selected(shown)
                    .flex_none()
                    .h(px(6.))
                    .w(px(6. + 10. * long))
                    .rounded_full()
                    .bg(mix(mix(p.line2, p.mute, long), p.acc, filled))
                    .test_support()
            })
            .collect();
        div()
            .id(self.id)
            .aria_label(self.label)
            .flex()
            .items_center()
            .gap(space::XS)
            .children(dots)
            .test_support()
    }
}
