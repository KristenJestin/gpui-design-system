//! What the scenario suites share: the kit initialised, and ways to count frames.
#![allow(dead_code)]

pub mod subpage;

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::{Entity, TestAppContext, VisualTestContext};

/// Initialises the kit for an application with its own look and no desktop source.
pub fn init(cx: &mut TestAppContext) {
    init_as(gpui_design_system::Identity::new("Kit"), cx);
}

/// [`init`] for the application `identity`.
pub fn init_as(identity: gpui_design_system::Identity, cx: &mut TestAppContext) {
    cx.update(|cx| gpui_design_system::init(identity, cx));
    cx.run_until_parked();
}

/// Counts every time `entity` asks to be redrawn: a timer tick, a state change, an animation
/// frame all go through `notify`.
pub fn count_redraws<T: 'static>(
    entity: &Entity<T>,
    cx: &mut VisualTestContext,
) -> Rc<Cell<usize>> {
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    cx.update(|_, cx| {
        cx.observe(entity, move |_, _| seen.set(seen.get() + 1))
            .detach();
    });
    count
}

pub fn advance(cx: &mut VisualTestContext, by: Duration) {
    let step = Duration::from_millis(16);
    let mut gone = Duration::ZERO;
    while gone < by {
        cx.executor().advance_clock(step);
        cx.update(|window, cx| window.simulate_next_frame(cx));
        cx.run_until_parked();
        gone += step;
    }
}

pub fn pending_frames(cx: &mut VisualTestContext) -> usize {
    cx.update(|window, cx| window.simulate_next_frame(cx))
}
