//! A `BulkBar`: a bar that unfolds when a selection is not empty and folds away when it empties,
//! on GPUI's test platform.

mod common;

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use common::{advance, init, pending_frames};
use gpui_design_system::motion::{HOVER, PANEL};
use gpui_design_system::{BulkBar, Button};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, Entity, InteractiveElement as _, IntoElement, ParentElement, Render, Styled,
    TestAppContext, TestSupportExt as _, VisualTestContext, Window, div, px,
};

const GAP: f32 = 12.;

struct Host {
    selected: usize,
    set: Rc<Cell<usize>>,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        let set = self.set.clone();
        div()
            .w(px(500.))
            .flex()
            .flex_col()
            .gap(px(GAP))
            .child(
                BulkBar::new("bar", self.selected)
                    .gap(px(GAP))
                    .action(Button::secondary("bar-off", "Off").on_click({
                        let set = set.clone();
                        move |_, _, _| set.set(set.get() + 1)
                    }))
                    .action(Button::secondary("bar-allowed", "Allowed"))
                    .on_clear(move |_, cx| {
                        this.update(cx, |this, cx| {
                            this.selected = 0;
                            cx.notify();
                        })
                        .ok();
                    }),
            )
            .child(div().id("after").h(px(20.)).test_support())
    }
}

fn host(
    selected: usize,
    reduce: bool,
    cx: &mut TestAppContext,
) -> (Entity<Host>, &mut VisualTestContext) {
    init(cx);
    cx.update(|cx| cx.set_reduce_motion(reduce));
    let (view, cx) = cx.add_window_view(move |_, _| Host {
        selected,
        set: Rc::default(),
    });
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, PANEL * 2);
    (view, cx)
}

fn select(view: &Entity<Host>, selected: usize, cx: &mut VisualTestContext) {
    view.update(cx, |this, cx| {
        this.selected = selected;
        cx.notify();
    });
    cx.update(|window, cx| window.render_frame(cx));
}

fn after(cx: &mut VisualTestContext) -> f32 {
    cx.update(|window, _| f32::from(window.find("after").bounds().origin.y))
}

fn label(id: &str, cx: &mut VisualTestContext) -> Option<String> {
    cx.update(|window, _| {
        window
            .try_find(id.to_owned())
            .and_then(|found| found.label().map(str::to_owned))
    })
}

#[gpui_kit::test]
fn the_bulk_bar_unfolds_with_a_selection_and_folds_without(cx: &mut TestAppContext) {
    let (view, cx) = host(0, false, cx);
    let none = after(cx);
    assert_eq!(
        label("bar-count", cx),
        None,
        "no bar while nothing is selected"
    );

    select(&view, 2, cx);
    advance(cx, PANEL / 3);
    let part = after(cx);
    assert!(none < part, "unfolding: {part}");
    advance(cx, PANEL * 2);
    let full = after(cx);
    assert!(full > part, "unfolded: {full}");
    assert_eq!(label("bar-count", cx), Some("2 selected".to_owned()));
    assert_eq!(label("bar-clear", cx), Some("Clear".to_owned()));
    assert_eq!(pending_frames(cx), 0);

    cx.update(|window, cx| window.click("bar-clear", cx));
    assert_eq!(view.read_with(cx, |this, _| this.selected), 0);
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, PANEL / 3);
    let folding = after(cx);
    assert!(none < folding && folding < full, "folding away: {folding}");
    assert_eq!(
        label("bar-count", cx),
        Some("2 selected".to_owned()),
        "it says what it held while it folds"
    );
    advance(cx, PANEL * 2);
    assert_eq!(after(cx), none, "no jump at the end");
    assert_eq!(label("bar-count", cx), None);
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn the_bulk_bar_runs_the_actions_it_was_given(cx: &mut TestAppContext) {
    let (view, cx) = host(3, false, cx);
    cx.update(|window, cx| window.click("bar-off", cx));
    assert_eq!(view.read_with(cx, |this, _| this.set.get()), 1);
}

#[gpui_kit::test]
fn the_bulk_bar_s_count_fades_in_when_it_changes(cx: &mut TestAppContext) {
    let (view, cx) = host(2, false, cx);
    assert_eq!(pending_frames(cx), 0);
    select(&view, 3, cx);
    assert!(pending_frames(cx) > 0, "the new count fades in");
    assert_eq!(label("bar-count", cx), Some("3 selected".to_owned()));
    advance(cx, HOVER * 2);
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn the_bulk_bar_jumps_with_reduced_motion(cx: &mut TestAppContext) {
    let (view, cx) = host(0, true, cx);
    let none = after(cx);
    select(&view, 2, cx);
    advance(cx, Duration::from_millis(16));
    assert!(after(cx) > none, "it is there at once");
    assert_eq!(pending_frames(cx), 0);
    select(&view, 0, cx);
    advance(cx, Duration::from_millis(16));
    assert_eq!(after(cx), none, "and gone at once");
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn the_space_the_bulk_bar_takes_gap_included_grows_over_the_fold(cx: &mut TestAppContext) {
    let (view, cx) = host(0, false, cx);
    let none = after(cx);

    select(&view, 2, cx);
    let spaces: Vec<f32> = std::iter::once(after(cx))
        .chain((0..(PANEL * 2).as_millis() / 16).map(|_| {
            advance(cx, Duration::from_millis(16));
            after(cx)
        }))
        .collect();
    assert!(
        (spaces[0] - none).abs() < 2.,
        "the gap does not appear at once: {spaces:?}"
    );
    assert!(
        spaces.windows(2).all(|pair| pair[0] <= pair[1]),
        "{spaces:?}"
    );
    assert!(
        spaces
            .iter()
            .any(|space| none < *space && *space < spaces[spaces.len() - 1])
    );
    let bar = cx.update(|window, _| f32::from(window.find("bar").bounds().size.height));
    let full = after(cx);
    assert!(
        (full - none - (bar + GAP)).abs() < 1.,
        "{full}: the bar {bar} and its gap"
    );

    select(&view, 0, cx);
    let spaces: Vec<f32> = std::iter::once(after(cx))
        .chain((0..(PANEL * 2).as_millis() / 16).map(|_| {
            advance(cx, Duration::from_millis(16));
            after(cx)
        }))
        .collect();
    assert!(
        (spaces[0] - full).abs() < 2.,
        "nor does it vanish at once: {spaces:?}"
    );
    assert!(
        spaces.windows(2).all(|pair| pair[0] >= pair[1]),
        "{spaces:?}"
    );
    assert_eq!(after(cx), none, "no jump at the end");
    assert_eq!(pending_frames(cx), 0);
}
