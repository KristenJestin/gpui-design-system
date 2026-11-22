//! The progress bar: a fill that eases to its share over `PANEL` and never goes back, a moving
//! stripe when no share is known, reduced motion that jumps, and no frame once it is at rest.

mod common;

use std::time::Duration;

use common::{advance, init, pending_frames};
use gpui_design_system::Progress;
use gpui_design_system::motion::PANEL;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, Entity, IntoElement, ParentElement, Pixels, Render, Styled, TestAppContext,
    VisualTestContext, Window, div, px,
};

const WIDTH: f32 = 200.;

struct Bar {
    /// `None` for a bar with no known share.
    share: Option<f32>,
}

impl Render for Bar {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let bar = match self.share {
            Some(share) => Progress::new("bar", share),
            None => Progress::indeterminate("bar"),
        };
        div().size_full().child(bar.w(px(WIDTH)))
    }
}

fn bar(share: Option<f32>, cx: &mut TestAppContext) -> (Entity<Bar>, &mut VisualTestContext) {
    init(cx);
    let (view, cx) = cx.add_window_view(move |_, _| Bar { share });
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

fn read(view: &Entity<Bar>, share: f32, cx: &mut VisualTestContext) {
    cx.update(|_, cx| {
        view.update(cx, |bar, cx| {
            bar.share = Some(share);
            cx.notify()
        })
    });
    cx.update(|window, cx| window.render_frame(cx));
}

fn width(id: &str, cx: &mut VisualTestContext) -> Option<Pixels> {
    let id = id.to_owned();
    cx.update(|window, _| window.try_find(id).map(|found| found.bounds().size.width))
}

/// A width is a float: it is the share's, up to the last bit.
fn is_about(width: Option<Pixels>, expected: f32) {
    let width = f32::from(width.expect("drawn"));
    assert!((width - expected).abs() < 0.01, "{width} is not {expected}");
}

fn left(id: &str, cx: &mut VisualTestContext) -> Pixels {
    let id = id.to_owned();
    cx.update(|window, _| window.find(id).bounds().origin.x)
}

#[gpui_kit::test]
fn a_bar_is_as_wide_as_it_is_told_and_fills_its_share(cx: &mut TestAppContext) {
    let (_, cx) = bar(Some(0.25), cx);
    assert_eq!(width("bar", cx), Some(px(WIDTH)));
    is_about(width("bar-fill", cx), WIDTH * 0.25);
}

#[gpui_kit::test]
fn a_bar_drawn_for_the_first_time_is_simply_at_its_share(cx: &mut TestAppContext) {
    let (_, cx) = bar(Some(0.6), cx);
    is_about(width("bar-fill", cx), WIDTH * 0.6);
    assert_eq!(pending_frames(cx), 0, "it did not start from nothing");
}

#[gpui_kit::test]
fn a_share_out_of_range_is_held_to_the_bar(cx: &mut TestAppContext) {
    let (view, cx) = bar(Some(1.7), cx);
    is_about(width("bar-fill", cx), WIDTH);
    read(&view, f32::NAN, cx);
    advance(cx, PANEL * 2);
    assert_eq!(
        width("bar-fill", cx),
        Some(px(WIDTH)),
        "nothing to go back to"
    );
}

#[gpui_kit::test]
fn the_fill_eases_to_a_new_share_over_the_panel_duration(cx: &mut TestAppContext) {
    let (view, cx) = bar(Some(0.2), cx);
    read(&view, 0.8, cx);
    advance(cx, PANEL / 2);
    let mid = f32::from(width("bar-fill", cx).unwrap()) / WIDTH;
    assert!(
        mid > 0.2 && mid < 0.8,
        "between two readings it is between the two shares: {mid}"
    );
    assert!(mid > 0.5, "ease-out is past half way at half time: {mid}");
    advance(cx, PANEL);
    is_about(width("bar-fill", cx), WIDTH * 0.8);
}

#[gpui_kit::test]
fn the_fill_never_jumps_back(cx: &mut TestAppContext) {
    let (view, cx) = bar(Some(0.7), cx);
    read(&view, 0.3, cx);
    assert_eq!(
        width("bar-fill", cx),
        Some(px(WIDTH * 0.7)),
        "a lower reading is ignored"
    );
    advance(cx, PANEL * 2);
    is_about(width("bar-fill", cx), WIDTH * 0.7);
    // Mid-run too: the fill only ever grows.
    read(&view, 0.9, cx);
    let mut last = 0.0;
    for _ in 0..30 {
        advance(cx, Duration::from_millis(16));
        let now = f32::from(width("bar-fill", cx).unwrap());
        assert!(now >= last, "the fill went back: {last} then {now}");
        last = now;
    }
}

#[gpui_kit::test]
fn a_bar_at_rest_asks_for_no_frame(cx: &mut TestAppContext) {
    let (view, cx) = bar(Some(0.2), cx);
    read(&view, 0.9, cx);
    advance(cx, PANEL * 2);
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn with_reduced_motion_the_fill_jumps(cx: &mut TestAppContext) {
    init(cx);
    cx.update(|cx| cx.set_reduce_motion(true));
    let (view, cx) = cx.add_window_view(|_, _| Bar { share: Some(0.2) });
    cx.update(|window, cx| window.render_frame(cx));
    read(&view, 0.9, cx);
    is_about(width("bar-fill", cx), WIDTH * 0.9);
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_bar_with_no_share_draws_a_stripe_narrower_than_itself(cx: &mut TestAppContext) {
    let (_, cx) = bar(None, cx);
    let stripe = width("bar-fill", cx).expect("a stripe");
    assert!(
        stripe > px(0.) && stripe < px(WIDTH),
        "a part of the bar: {stripe:?}"
    );
}

#[gpui_kit::test]
fn the_stripe_moves_along_the_bar_and_stays_inside_it(cx: &mut TestAppContext) {
    let (_, cx) = bar(None, cx);
    let track = left("bar", cx);
    let mut seen = Vec::new();
    for _ in 0..40 {
        advance(cx, Duration::from_millis(48));
        let at = left("bar-fill", cx) - track;
        seen.push(f32::from(at));
        assert!(
            at >= px(-WIDTH) && at <= px(WIDTH),
            "the stripe is on its way across the bar: {at:?}"
        );
    }
    let distinct = seen.windows(2).filter(|pair| pair[0] != pair[1]).count();
    assert!(distinct > 20, "it moves: {seen:?}");
    assert!(
        pending_frames(cx) > 0,
        "it keeps asking for frames while it is drawn"
    );
}

#[gpui_kit::test]
fn with_reduced_motion_the_stripe_stands_still_and_asks_for_no_frame(cx: &mut TestAppContext) {
    init(cx);
    cx.update(|cx| cx.set_reduce_motion(true));
    let (_, cx) = cx.add_window_view(|_, _| Bar { share: None });
    cx.update(|window, cx| window.render_frame(cx));
    let first = left("bar-fill", cx);
    advance(cx, Duration::from_millis(600));
    assert_eq!(left("bar-fill", cx), first);
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_bar_that_learns_its_size_goes_from_the_stripe_to_a_fill(cx: &mut TestAppContext) {
    let (view, cx) = bar(None, cx);
    read(&view, 0.5, cx);
    advance(cx, PANEL * 2);
    is_about(width("bar-fill", cx), WIDTH * 0.5);
    assert_eq!(pending_frames(cx), 0);
}
