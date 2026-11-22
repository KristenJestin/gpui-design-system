//! How far a transfer has come: a [`Progress`] bar whose fill eases to each new share over
//! [`PANEL`] and never goes back, or, when no share is known, a stripe that travels along it.
//!
//! At rest a bar draws no frame: only a fill that is on its way, or a stripe, asks for the next
//! one. With reduced motion the fill jumps and the stripe stands still.

use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use gpui_kit::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, Pixels, RenderOnce, Styled,
    TestSupportExt, Window, div, px, relative,
};

use crate::ids::part;
use crate::look::{Radius, Shaped as _, Surface};
use crate::motion::{Motion, PANEL};
use crate::theme::palette;

/// How long the stripe takes to cross the bar.
const SWEEP: Duration = Duration::from_millis(1400);
/// The stripe's width, as a share of the bar's.
const STRIPE: f32 = 0.35;

/// How far along the bar (as a share of its width) the left edge of the stripe is, `elapsed` into
/// the clock all stripes share, so that those drawn together move together: from just out of the
/// bar on its left to just out of it on its right.
pub fn stripe_start(elapsed: Duration) -> f32 {
    let phase = (elapsed.as_secs_f32() / SWEEP.as_secs_f32()).fract();
    -STRIPE + (1.0 + STRIPE) * phase
}

/// The instant all stripes count from.
fn epoch(now: Instant) -> Instant {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    *EPOCH.get_or_init(|| now)
}

/// A bar for a download or a transfer: `Progress::new(id, share)` with the share received
/// (0 to 1), `Progress::indeterminate(id)` when the size is not known. Its width is the room it
/// is in unless [`w`](Self::w) says otherwise. A test finds the bar as `<id>` and its fill, or
/// stripe, as `<id>-fill`.
///
/// Build it on every frame with the latest share: it is its own state. A share lower than one it
/// already showed is ignored, so the fill never goes back; a new transfer has a new id.
#[derive(IntoElement)]
pub struct Progress {
    id: ElementId,
    share: Option<f32>,
    width: Option<Pixels>,
}

impl Progress {
    /// A bar filled to `share`, from 0 (nothing) to 1 (all); anything else is held to that range.
    pub fn new(id: impl Into<ElementId>, share: f32) -> Self {
        let share = if share.is_finite() {
            share.clamp(0.0, 1.0)
        } else {
            0.0
        };
        Self {
            id: id.into(),
            share: Some(share),
            width: None,
        }
    }

    /// A bar of a transfer whose size is not known: a stripe moves along it.
    pub fn indeterminate(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            share: None,
            width: None,
        }
    }

    pub fn w(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }
}

impl RenderOnce for Progress {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let key = ElementId::NamedChild(Arc::new(self.id.clone()), "progress".into());
        let first = self.share.unwrap_or(0.0);
        let state = window.use_keyed_state(key, cx, move |_, _| Motion::settled(first));
        let now = cx.background_executor().now();
        let reduce = cx.reduce_motion();
        let fill_id = part(&self.id, "fill");
        let fill = match self.share {
            Some(share) => {
                let motion = state.update(cx, |motion, _| {
                    // Never back: a reading below the one shown is a late or stale one.
                    let target = share.max(motion.target());
                    if motion.target() != target {
                        motion.animate(target, PANEL, now, reduce);
                    }
                    *motion
                });
                if motion.is_running(now) {
                    window.request_animation_frame();
                } else {
                    state.update(cx, |motion, _| motion.settle(now));
                }
                div()
                    .id(fill_id)
                    .h_full()
                    .w(relative(motion.value(now)))
                    .corners(Radius::Full)
                    .bg(p.acc)
                    .test_support()
            }
            None => {
                // A stripe that stands still sits in the middle, where it reads as a bar.
                let start = match reduce {
                    true => (1.0 - STRIPE) / 2.0,
                    false => {
                        window.request_animation_frame();
                        stripe_start(now.saturating_duration_since(epoch(now)))
                    }
                };
                div()
                    .id(fill_id)
                    .absolute()
                    .top_0()
                    .h_full()
                    .left(relative(start))
                    .w(relative(STRIPE))
                    .corners(Radius::Full)
                    .bg(p.acc)
                    .test_support()
            }
        };
        let track = div().id(self.id).relative().h(px(6.)).overflow_hidden();
        match self.width {
            Some(width) => track.w(width),
            None => track.w_full(),
        }
        .surface(Surface::full().fill(p.line2))
        .child(fill)
        .test_support()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stripe_crosses_the_bar_once_every_sweep() {
        let at = |ms: u64| stripe_start(Duration::from_millis(ms));
        assert!(
            (at(0) + STRIPE).abs() < 1e-4,
            "it starts out of the bar on the left"
        );
        assert!(at(700) > at(0) && at(1000) > at(700));
        assert!(at(1399) < 1.0, "and has not yet left it on the right");
        assert!((at(1400) + STRIPE).abs() < 1e-4, "then starts again");
    }
}
