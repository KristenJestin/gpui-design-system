//! A list that opens at rest: the rows of its first `sync` are drawn at their full height from the
//! first frame, with no entrance and no stagger (a dialog that opens on data it already has and
//! fades in as a whole), while the rows that come, go or move afterwards unfold, fold and slide as
//! in every other list, on GPUI's test platform.

mod common;

use common::{advance, init, pending_frames};
use gpui_design_system::motion::{PANEL, STAGGER};
use gpui_design_system::{Grouped, Item, Listing};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, Entity, IntoElement, ParentElement, Pixels, Render, Styled, TestAppContext,
    VisualTestContext, Window, div, px,
};

const ROW: f32 = 40.;
const GAP: f32 = 8.;

// ── A list ─────────────────────────────────────────────────────────────────────────────────────

struct Rows {
    list: Listing<&'static str>,
    items: Vec<&'static str>,
}

impl Render for Rows {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = cx.background_executor().now();
        self.list.sync(
            self.items.iter().map(|name| (name.to_string(), *name)),
            now,
            cx.reduce_motion(),
        );
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(GAP))
            .children(self.list.rows().map(|row| {
                Item::new(format!("item-{}", row.key), row.placement, div().h(px(ROW))).gap(px(GAP))
            }))
    }
}

/// A list drawn once, with no time passed since its first frame.
fn first_frame<'a>(
    list: Listing<&'static str>,
    items: &[&'static str],
    cx: &'a mut TestAppContext,
) -> (Entity<Rows>, &'a mut VisualTestContext) {
    init(cx);
    let items = items.to_vec();
    let (view, cx) = cx.add_window_view(move |_, _| Rows { list, items });
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

fn change(view: &Entity<Rows>, items: &[&'static str], cx: &mut VisualTestContext) {
    let items = items.to_vec();
    cx.update(|_, cx| {
        view.update(cx, |rows, cx| {
            rows.items = items;
            cx.notify();
        })
    });
    cx.update(|window, cx| window.render_frame(cx));
}

fn height(id: &str, cx: &mut VisualTestContext) -> Option<Pixels> {
    let id = id.to_owned();
    cx.update(|window, _| window.try_find(id).map(|found| found.bounds().size.height))
}

#[gpui_kit::test]
fn a_list_at_rest_draws_its_first_rows_at_full_height_on_the_first_frame(cx: &mut TestAppContext) {
    // Three rows: the third would wait two staggers if it entered.
    let (_, cx) = first_frame(Listing::default().at_rest(), &["a", "b", "c"], cx);
    for id in ["item-a", "item-b", "item-c"] {
        assert_eq!(height(id, cx), Some(px(ROW)), "{id} is at its height");
    }
    assert_eq!(
        pending_frames(cx),
        0,
        "no frame is asked for once it is drawn: nothing plays"
    );
}

#[gpui_kit::test]
fn a_list_at_rest_asks_for_no_frame_while_time_passes(cx: &mut TestAppContext) {
    let (_, cx) = first_frame(Listing::default().at_rest(), &["a", "b"], cx);
    advance(cx, PANEL);
    assert_eq!(height("item-a", cx), Some(px(ROW)));
    assert_eq!(height("item-b", cx), Some(px(ROW)));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_row_added_to_a_list_at_rest_still_unfolds(cx: &mut TestAppContext) {
    let (view, cx) = first_frame(Listing::default().at_rest(), &["a", "b"], cx);
    change(&view, &["a", "b", "c"], cx);
    advance(cx, PANEL / 3);
    let part = height("item-c", cx).expect("the new row is drawn");
    assert!(
        px(0.) < part && part < px(ROW),
        "part way after a third: {part:?}"
    );
    assert_eq!(
        height("item-a", cx),
        Some(px(ROW)),
        "the rows that were there stay put"
    );
    advance(cx, PANEL * 2);
    assert_eq!(height("item-c", cx), Some(px(ROW)));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_row_of_a_list_at_rest_folds_away_when_removed(cx: &mut TestAppContext) {
    let (view, cx) = first_frame(Listing::default().at_rest(), &["a", "b", "c"], cx);
    change(&view, &["a", "c"], cx);
    advance(cx, PANEL / 3);
    let part = height("item-b", cx).expect("the removed row is still drawn while it folds");
    assert!(
        px(0.) < part && part < px(ROW + GAP),
        "folding, part way: {part:?}"
    );
    advance(cx, PANEL * 2);
    assert_eq!(height("item-b", cx), None, "gone once folded");
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_row_of_a_list_at_rest_slides_when_it_moves(cx: &mut TestAppContext) {
    let (view, cx) = first_frame(Listing::default().at_rest(), &["a", "b"], cx);
    let top = |id: &str, cx: &mut VisualTestContext| {
        let id = id.to_owned();
        cx.update(|window, _| window.find(id).bounds().origin.y)
    };
    let (a, b) = (top("item-a", cx), top("item-b", cx));
    change(&view, &["b", "a"], cx);
    advance(cx, PANEL / 3);
    let sliding = top("item-a", cx);
    assert!(
        a < sliding && sliding < b,
        "a is on its way down: {a:?} < {sliding:?} < {b:?}"
    );
    advance(cx, PANEL * 2);
    assert_eq!((top("item-b", cx), top("item-a", cx)), (a, b));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_list_without_at_rest_keeps_its_staggered_entrance(cx: &mut TestAppContext) {
    let (_, cx) = first_frame(Listing::default(), &["a", "b", "c"], cx);
    assert_eq!(
        height("item-a", cx).unwrap_or(px(0.)),
        px(0.),
        "the first row has not started on the first frame"
    );
    assert!(pending_frames(cx) > 0, "it is playing");
    // The first row is on its way while the third still waits its two staggers.
    advance(cx, STAGGER);
    let first = height("item-a", cx).expect("the first row is drawn");
    assert!(
        px(0.) < first && first < px(ROW),
        "a is part way: {first:?}"
    );
    assert_eq!(
        height("item-c", cx).unwrap_or(px(0.)),
        px(0.),
        "c is still waiting for its turn"
    );
    advance(cx, PANEL * 3);
    for id in ["item-a", "item-b", "item-c"] {
        assert_eq!(height(id, cx), Some(px(ROW)), "{id} arrived");
    }
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_list_at_rest_that_starts_empty_unfolds_the_rows_that_come_after(cx: &mut TestAppContext) {
    // Only the first sync is at rest: a list that opens empty has no row to place.
    let (view, cx) = first_frame(Listing::default().at_rest(), &[], cx);
    change(&view, &["a"], cx);
    advance(cx, PANEL / 3);
    let part = height("item-a", cx).expect("the row is drawn");
    assert!(px(0.) < part && part < px(ROW), "part way: {part:?}");
    advance(cx, PANEL * 2);
    assert_eq!(height("item-a", cx), Some(px(ROW)));
}

// ── Grouped rows ───────────────────────────────────────────────────────────────────────────────

const GROUPS: [&str; 2] = ["one", "two"];

struct Several {
    grouped: Grouped<&'static str, &'static str>,
    /// Each row: its key and its group.
    rows: Vec<(&'static str, &'static str)>,
}

impl Render for Several {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = cx.background_executor().now();
        self.grouped.sync(
            GROUPS,
            self.rows
                .iter()
                .map(|(key, group)| (key.to_string(), *group, *key)),
            now,
            cx.reduce_motion(),
        );
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(GAP))
            .children(self.grouped.groups().map(|list| {
                div().flex().flex_col().gap(px(GAP)).children(
                    list.rows().map(|row| {
                        Item::new(row.id(), row.placement, div().h(px(ROW))).gap(px(GAP))
                    }),
                )
            }))
    }
}

fn grouped_first_frame<'a>(
    grouped: Grouped<&'static str, &'static str>,
    rows: &[(&'static str, &'static str)],
    cx: &'a mut TestAppContext,
) -> (Entity<Several>, &'a mut VisualTestContext) {
    init(cx);
    let rows = rows.to_vec();
    let (view, cx) = cx.add_window_view(move |_, _| Several { grouped, rows });
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

fn regroup(
    view: &Entity<Several>,
    rows: &[(&'static str, &'static str)],
    cx: &mut VisualTestContext,
) {
    let rows = rows.to_vec();
    cx.update(|_, cx| {
        view.update(cx, |several, cx| {
            several.rows = rows;
            cx.notify();
        })
    });
    cx.update(|window, cx| window.render_frame(cx));
}

#[gpui_kit::test]
fn grouped_rows_at_rest_are_drawn_at_full_height_on_the_first_frame(cx: &mut TestAppContext) {
    let (_, cx) = grouped_first_frame(
        Grouped::new("rows").at_rest(),
        &[("a", "one"), ("b", "one"), ("c", "two")],
        cx,
    );
    for id in ["rows-0-a", "rows-0-b", "rows-1-c"] {
        assert_eq!(height(id, cx), Some(px(ROW)), "{id} is at its height");
    }
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_row_added_to_grouped_rows_at_rest_still_unfolds(cx: &mut TestAppContext) {
    let (view, cx) = grouped_first_frame(Grouped::new("rows").at_rest(), &[("a", "one")], cx);
    regroup(&view, &[("a", "one"), ("b", "two")], cx);
    advance(cx, PANEL / 3);
    let part = height("rows-1-b", cx).expect("the new row is drawn");
    assert!(px(0.) < part && part < px(ROW), "part way: {part:?}");
    advance(cx, PANEL * 2);
    assert_eq!(height("rows-1-b", cx), Some(px(ROW)));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_row_that_changes_group_leaves_one_at_rest_and_unfolds_in_the_other(cx: &mut TestAppContext) {
    let (view, cx) = grouped_first_frame(
        Grouped::new("rows").at_rest(),
        &[("a", "one"), ("b", "one")],
        cx,
    );
    regroup(&view, &[("a", "one"), ("b", "two")], cx);
    advance(cx, PANEL / 3);
    let leaving = height("rows-0-b", cx).expect("it is still drawn where it was");
    assert!(px(0.) < leaving && leaving < px(ROW + GAP), "{leaving:?}");
    let arriving = height("rows-1-b", cx).expect("it is drawn where it goes");
    assert!(px(0.) < arriving && arriving < px(ROW), "{arriving:?}");
    advance(cx, PANEL * 2);
    assert_eq!(height("rows-0-b", cx), None);
    assert_eq!(height("rows-1-b", cx), Some(px(ROW)));
}

#[gpui_kit::test]
fn grouped_rows_without_at_rest_keep_their_staggered_entrance(cx: &mut TestAppContext) {
    let (_, cx) = grouped_first_frame(
        Grouped::new("rows"),
        &[("a", "one"), ("b", "one"), ("c", "two")],
        cx,
    );
    assert_eq!(height("rows-0-a", cx).unwrap_or(px(0.)), px(0.));
    assert!(pending_frames(cx) > 0, "it is playing");
    advance(cx, PANEL * 3);
    assert_eq!(height("rows-1-c", cx), Some(px(ROW)));
    assert_eq!(pending_frames(cx), 0);
}
