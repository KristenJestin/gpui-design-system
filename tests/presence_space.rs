//! A block that comes and goes holds its own folding space: `presence(..).space(..)` is a margin
//! above the block, folded with it, so it needs nothing of its parent. In a column with no gap
//! the space unfolds and folds smoothly instead of jumping at the first and last frame; in a
//! column with a gap, the old `.gap(..)` call is drawn the same way it always was.

mod common;

use std::time::Duration;

use common::{advance, init, pending_frames};
use gpui_design_system::motion::PANEL;
use gpui_design_system::{presence, presence_of};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, InteractiveElement as _, IntoElement, ParentElement, Pixels, Render, SharedString,
    Styled, TestAppContext, TestSupportExt as _, VisualTestContext, Window, div, px,
};

const SPACE: f32 = 10.;
const BLOCK: f32 = 60.;
const BEFORE: f32 = 20.;

/// How the space above the block is asked for.
#[derive(Clone, Copy, PartialEq)]
enum Space {
    /// The block's own margin, in a column with no gap.
    Own,
    /// The column's gap, which the block folds with itself (`.gap(..)`).
    Borrowed,
}

/// Which block comes and goes.
#[derive(Clone, Copy, PartialEq)]
enum Block {
    Presence,
    PresenceOf,
}

struct Subject {
    space: Space,
    block: Block,
    shown: bool,
}

impl Render for Subject {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let column = div().size_full().flex().flex_col();
        let column = match self.space {
            Space::Own => column,
            Space::Borrowed => column.gap(px(SPACE)),
        };
        let content = |id: &'static str| div().id(id).h(px(BLOCK)).test_support();
        let column = column.child(div().id("before").h(px(BEFORE)).test_support());
        let column = match (self.block, self.space) {
            (Block::Presence, Space::Own) => {
                column.child(presence("block", self.shown, content("content")).space(px(SPACE)))
            }
            (Block::Presence, Space::Borrowed) => {
                column.child(presence("block", self.shown, content("content")).gap(px(SPACE)))
            }
            (Block::PresenceOf, space) => {
                let line = self
                    .shown
                    .then(|| SharedString::from("Refused: no such folder"));
                let block = presence_of("block", line, move |_| {
                    content("content").into_any_element()
                });
                column.child(match space {
                    Space::Own => block.space(px(SPACE)),
                    Space::Borrowed => block.gap(px(SPACE)),
                })
            }
        };
        column.child(div().id("after").h(px(BEFORE)).test_support())
    }
}

fn subject(
    space: Space,
    block: Block,
    shown: bool,
    cx: &mut TestAppContext,
) -> (gpui_kit::Entity<Subject>, &mut VisualTestContext) {
    init(cx);
    let (view, cx) = cx.add_window_view(move |_, _| Subject {
        space,
        block,
        shown,
    });
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

fn show(view: &gpui_kit::Entity<Subject>, shown: bool, cx: &mut VisualTestContext) {
    cx.update(|_, cx| {
        view.update(cx, |subject, cx| {
            subject.shown = shown;
            cx.notify()
        })
    });
    cx.update(|window, cx| window.render_frame(cx));
}

fn top(id: &str, cx: &mut VisualTestContext) -> Pixels {
    let id = id.to_owned();
    cx.update(|window, _| window.find(id).bounds().origin.y)
}

/// The space between the line above the block and the block, as drawn now.
fn space_above(cx: &mut VisualTestContext) -> f32 {
    f32::from(top("content", cx) - top("before", cx)) - BEFORE
}

/// What `after` is moved down by since the block was last absent, frame after frame over
/// `PANEL`, starting with the frame the change is drawn in.
fn trace(
    view: &gpui_kit::Entity<Subject>,
    shown: bool,
    closed: Pixels,
    cx: &mut VisualTestContext,
) -> Vec<f32> {
    show(view, shown, cx);
    let step = Duration::from_millis(16);
    let mut moved = vec![f32::from(top("after", cx) - closed)];
    let mut gone = Duration::ZERO;
    while gone < PANEL + step * 2 {
        advance(cx, step);
        moved.push(f32::from(top("after", cx) - closed));
        gone += step;
    }
    moved
}

/// The unfolding then the folding away of a block, as `after` is moved by them.
fn there_and_back(space: Space, block: Block, cx: &mut TestAppContext) -> (Vec<f32>, Vec<f32>) {
    let (view, cx) = subject(space, block, false, cx);
    let closed = top("after", cx);
    let unfolding = trace(&view, true, closed, cx);
    let folding = trace(&view, false, closed, cx);
    assert_eq!(pending_frames(cx), 0);
    (unfolding, folding)
}

/// A trace that starts at `from` px moved, ends at `to` and never jumps in between.
fn assert_smooth(moved: &[f32], from: f32, to: f32, what: &str) {
    assert!(
        (moved[0] - from).abs() < 0.01,
        "{what}: the frame the change is drawn in is still where it was: {}",
        moved[0]
    );
    for pair in moved.windows(2) {
        let step = (pair[1] - pair[0]).abs();
        assert!(
            step < (BLOCK + SPACE) / 2.,
            "{what}: no jump of {step} between two frames: {moved:?}"
        );
    }
    let last = *moved.last().expect("frames");
    assert!((last - to).abs() < 0.01, "{what}: ends at rest: {last}");
}

fn assert_smooth_both_ways(unfolding: &[f32], folding: &[f32]) {
    assert_smooth(unfolding, 0., BLOCK + SPACE, "unfolding");
    assert_smooth(folding, BLOCK + SPACE, 0., "folding away");
}

#[gpui_kit::test]
fn a_presence_in_a_column_without_a_gap_folds_its_own_space_smoothly(cx: &mut TestAppContext) {
    let (view, cx) = subject(Space::Own, Block::Presence, false, cx);
    let found = cx.update(|window, _| window.try_find("content").is_some());
    assert!(!found, "absent at rest: nothing is drawn");

    show(&view, true, cx);
    assert!(
        space_above(cx).abs() < 0.01,
        "the space starts from nothing, not from its full size"
    );
    advance(cx, PANEL / 3);
    let part = space_above(cx);
    assert!(0. < part && part < SPACE, "part way: {part}");
    advance(cx, PANEL * 2);
    assert!(
        (space_above(cx) - SPACE).abs() < 0.01,
        "its whole space once open"
    );

    show(&view, false, cx);
    advance(cx, PANEL / 3);
    let part = space_above(cx);
    assert!(0. < part && part < SPACE, "folding away, part way: {part}");
    advance(cx, PANEL * 2);
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_presence_in_a_column_without_a_gap_moves_what_follows_without_a_jump(cx: &mut TestAppContext) {
    let (unfolding, folding) = there_and_back(Space::Own, Block::Presence, cx);
    assert_smooth_both_ways(&unfolding, &folding);
}

#[gpui_kit::test]
fn a_presence_of_a_line_in_a_column_without_a_gap_folds_its_own_space_smoothly(
    cx: &mut TestAppContext,
) {
    let (unfolding, folding) = there_and_back(Space::Own, Block::PresenceOf, cx);
    assert_smooth_both_ways(&unfolding, &folding);
}

#[gpui_kit::test]
fn a_presence_in_a_column_with_a_gap_looks_the_same_with_its_own_space(cx: &mut TestAppContext) {
    for block in [Block::Presence, Block::PresenceOf] {
        let (borrowed_in, borrowed_out) = there_and_back(Space::Borrowed, block, cx);
        let (own_in, own_out) = there_and_back(Space::Own, block, cx);
        assert_eq!(borrowed_in.len(), own_in.len());
        for (old, new) in borrowed_in.iter().zip(&own_in) {
            assert!(
                (old - new).abs() < 0.01,
                "unfolding: {borrowed_in:?} then {own_in:?}"
            );
        }
        for (old, new) in borrowed_out.iter().zip(&own_out) {
            assert!(
                (old - new).abs() < 0.01,
                "folding: {borrowed_out:?} then {own_out:?}"
            );
        }
    }
}

#[gpui_kit::test]
fn a_presence_with_reduced_motion_jumps_to_its_own_space(cx: &mut TestAppContext) {
    init(cx);
    cx.update(|cx| cx.set_reduce_motion(true));
    let (view, cx) = subject(Space::Own, Block::Presence, false, cx);
    let closed = top("after", cx);
    show(&view, true, cx);
    advance(cx, Duration::from_millis(16));
    assert_eq!(top("after", cx), closed + px(BLOCK + SPACE));
    show(&view, false, cx);
    advance(cx, Duration::from_millis(16));
    assert_eq!(top("after", cx), closed);
}
