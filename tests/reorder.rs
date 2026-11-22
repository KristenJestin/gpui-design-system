//! A `Reorderable` list: rows dragged to their place, gathered in foldable groups that rows and
//! groups are dragged into and among, on GPUI's test platform.

mod common;

use common::{advance, init, pending_frames};
use gpui_design_system::motion::{HOVER, PANEL};
use gpui_design_system::{ReorderGroup, Reorderable};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Bounds, Context, Entity, InteractiveElement as _, IntoElement, Modifiers, MouseButton,
    ParentElement, Pixels, Point, Render, StatefulInteractiveElement as _, Styled, TestAppContext,
    TestSupportExt as _, VisualTestContext, Window, div, point, px,
};

const ROW: f32 = 30.;
const GAP: f32 = 4.;
const PITCH: f32 = ROW + GAP;

struct Group {
    key: &'static str,
    open: bool,
    rows: Vec<&'static str>,
}

struct Host {
    groups: Vec<Group>,
    loose: Vec<&'static str>,
    /// What the list said, in order.
    said: Vec<String>,
    clicks: Vec<&'static str>,
    /// Whether the host applies a move it is told of: a refusal leaves the order as it was.
    accepts: bool,
}

fn take(host: &mut Host, row: &str) -> &'static str {
    let key = host
        .groups
        .iter()
        .flat_map(|g| g.rows.iter())
        .chain(host.loose.iter())
        .copied()
        .find(|k| *k == row)
        .expect("a known row");
    host.groups
        .iter_mut()
        .for_each(|g| g.rows.retain(|k| *k != row));
    host.loose.retain(|k| *k != row);
    key
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let body = |key: &'static str, cx: &mut Context<Self>| {
            div()
                .id(format!("body-{key}"))
                .h(px(ROW))
                .w_full()
                .on_click(cx.listener(move |this, _, _, _| this.clicks.push(key)))
                .child(key)
        };
        let mut list = Reorderable::new("list").gap(px(GAP));
        for g in &self.groups {
            let mut group = ReorderGroup::new(g.key, g.key.to_uppercase()).open(g.open);
            group = group.mark(
                div()
                    .id(format!("mark-{}", g.key))
                    .w(px(12.))
                    .h(px(12.))
                    .test_support(),
            );
            for key in &g.rows {
                group = group.row(*key, body(key, cx));
            }
            list = list.group(group);
        }
        for key in &self.loose {
            list = list.row(*key, body(key, cx));
        }
        let (rows, groups, toggles) = (view.clone(), view.clone(), view);
        div().w(px(400.)).p(px(10.)).child(
            list.on_move_row(move |m, _, cx| {
                rows.update(cx, |this, cx| {
                    this.said.push(format!(
                        "row {} -> {} @{}",
                        m.row,
                        m.group.as_deref().unwrap_or("-"),
                        m.index
                    ));
                    if this.accepts {
                        let key = take(this, &m.row);
                        match m.group.as_deref() {
                            Some(g) => this
                                .groups
                                .iter_mut()
                                .find(|group| group.key == g)
                                .unwrap()
                                .rows
                                .insert(m.index, key),
                            None => this.loose.insert(m.index, key),
                        }
                    }
                    cx.notify();
                })
            })
            .on_move_group(move |key, index, _, cx| {
                groups.update(cx, |this, cx| {
                    this.said.push(format!("group {key} @{index}"));
                    if this.accepts {
                        let at = this.groups.iter().position(|g| g.key == key).unwrap();
                        let group = this.groups.remove(at);
                        this.groups.insert(index, group);
                    }
                    cx.notify();
                })
            })
            .on_toggle(move |key, open, _, cx| {
                toggles.update(cx, |this, cx| {
                    this.said.push(format!("toggle {key} {open}"));
                    this.groups.iter_mut().find(|g| g.key == key).unwrap().open = open;
                    cx.notify();
                })
            }),
        )
    }
}

fn host<'a>(
    groups: &[(&'static str, bool, &[&'static str])],
    loose: &[&'static str],
    reduce: bool,
    cx: &'a mut TestAppContext,
) -> (Entity<Host>, &'a mut VisualTestContext) {
    init(cx);
    cx.update(|cx| cx.set_reduce_motion(reduce));
    let groups = groups
        .iter()
        .map(|(key, open, rows)| Group {
            key,
            open: *open,
            rows: rows.to_vec(),
        })
        .collect();
    let loose = loose.to_vec();
    let (view, cx) = cx.add_window_view(move |_, _| Host {
        groups,
        loose,
        said: Vec::new(),
        clicks: Vec::new(),
        accepts: true,
    });
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, PANEL * 2);
    (view, cx)
}

fn bounds(id: &str, cx: &mut VisualTestContext) -> Bounds<Pixels> {
    let id = id.to_owned();
    cx.update(|window, _| window.find(id).bounds())
}

fn top(key: &str, cx: &mut VisualTestContext) -> f32 {
    f32::from(bounds(&format!("list-row-{key}"), cx).origin.y)
}

fn shown(id: &str, cx: &mut VisualTestContext) -> bool {
    cx.update(|window, _| window.try_find(id.to_owned()).is_some())
}

fn said(view: &Entity<Host>, cx: &mut VisualTestContext) -> Vec<String> {
    view.read_with(cx, |this, _| this.said.clone())
}

fn value(id: &str) -> Option<f32> {
    gpui_design_system::seen::seen(id.to_owned())
}

const NONE: Modifiers = Modifiers {
    control: false,
    alt: false,
    shift: false,
    platform: false,
    function: false,
};

fn press(at: Point<Pixels>, cx: &mut VisualTestContext) {
    cx.simulate_mouse_move(at, None, NONE);
    cx.simulate_mouse_down(at, MouseButton::Left, NONE);
    advance(cx, FRAME);
}

const FRAME: std::time::Duration = std::time::Duration::from_millis(16);

/// Moves the held pointer to `to` in small steps, drawing a frame after each.
fn pull(from: Point<Pixels>, to: Point<Pixels>, cx: &mut VisualTestContext) {
    let steps = 6;
    for step in 1..=steps {
        let t = step as f32 / steps as f32;
        let at = point(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
        cx.simulate_mouse_move(at, Some(MouseButton::Left), NONE);
        advance(cx, FRAME);
    }
}

fn release(at: Point<Pixels>, cx: &mut VisualTestContext) {
    cx.simulate_mouse_up(at, MouseButton::Left, NONE);
    advance(cx, FRAME);
}

/// Presses the row `id` and pulls it so that its centre is at `centre`, still held.
fn lift(id: &str, centre: f32, cx: &mut VisualTestContext) -> (Point<Pixels>, Point<Pixels>) {
    let from = bounds(id, cx).center();
    press(from, cx);
    let to = point(from.x, px(centre));
    pull(from, to, cx);
    (from, to)
}

fn drag(id: &str, centre: f32, cx: &mut VisualTestContext) {
    let (_, to) = lift(id, centre, cx);
    release(to, cx);
}

fn settle(cx: &mut VisualTestContext) {
    advance(cx, PANEL * 2);
}

fn centre(key: &str, cx: &mut VisualTestContext) -> f32 {
    f32::from(bounds(&format!("list-row-{key}"), cx).center().y)
}

#[gpui_kit::test]
fn a_row_is_dragged_to_its_place_and_the_list_says_the_new_order(cx: &mut TestAppContext) {
    let (view, cx) = host(&[], &["a", "b", "c", "d"], false, cx);
    let (a, c) = (top("a", cx), centre("c", cx));
    assert_eq!(top("c", cx), a + 2. * PITCH);

    drag("list-row-a", c + 5., cx);
    assert_eq!(said(&view, cx), ["row a -> - @2"]);
    settle(cx);
    let order: Vec<f32> = ["b", "c", "a", "d"].map(|k| top(k, cx)).to_vec();
    assert_eq!(order, [a, a + PITCH, a + 2. * PITCH, a + 3. * PITCH]);
    assert_eq!(pending_frames(cx), 0, "nothing moves at rest");
}

#[gpui_kit::test]
fn the_rows_around_slide_aside_while_the_row_is_held(cx: &mut TestAppContext) {
    let (view, cx) = host(&[], &["a", "b", "c", "d"], false, cx);
    let (a, b, c, d) = (top("a", cx), top("b", cx), top("c", cx), top("d", cx));
    let to = centre("c", cx) + 5.;

    let (from, held) = lift("list-row-a", to, cx);
    advance(cx, PANEL * 2);
    assert_eq!(top("b", cx), b - PITCH, "b made way");
    assert_eq!(top("c", cx), c - PITCH, "c made way");
    assert_eq!(top("d", cx), d, "d is out of the way");
    let held_at = top("a", cx);
    assert!(
        (held_at - (a + f32::from(held.y - from.y))).abs() < 1.,
        "the row follows the pointer: {held_at}"
    );
    assert!(
        said(&view, cx).is_empty(),
        "nothing is said before the drop"
    );
    release(held, cx);
}

#[gpui_kit::test]
fn the_rows_slide_over_the_panel_time_and_not_at_once(cx: &mut TestAppContext) {
    let (_, cx) = host(&[], &["a", "b", "c"], false, cx);
    let b = top("b", cx);
    let to = centre("b", cx) + 5.;
    let from = bounds("list-row-a", cx).center();
    press(from, cx);
    // A single jump of the pointer, then a frame at a time.
    cx.simulate_mouse_move(point(from.x, px(to)), Some(MouseButton::Left), NONE);
    advance(cx, PANEL / 3);
    let mid = top("b", cx);
    assert!(b - PITCH < mid && mid < b, "b is on its way: {mid}");
    advance(cx, PANEL * 2);
    assert_eq!(top("b", cx), b - PITCH);
    release(point(from.x, px(to)), cx);
}

#[gpui_kit::test]
fn a_dashed_slot_shows_where_the_row_lands(cx: &mut TestAppContext) {
    let (_, cx) = host(&[], &["a", "b", "c", "d"], false, cx);
    let a = top("a", cx);
    assert!(!shown("list-slot", cx), "no slot at rest");
    let to = centre("c", cx) + 5.;
    let (_, held) = lift("list-row-a", to, cx);
    advance(cx, PANEL * 2);
    let slot = bounds("list-slot", cx);
    assert_eq!(f32::from(slot.origin.y), a + 2. * PITCH);
    assert_eq!(f32::from(slot.size.height), ROW);
    release(held, cx);
    settle(cx);
    assert!(
        !shown("list-slot", cx),
        "the slot is gone once the row settled"
    );
}

#[gpui_kit::test]
fn a_press_that_does_not_move_is_a_click_and_a_drag_is_not(cx: &mut TestAppContext) {
    let (view, cx) = host(&[], &["a", "b", "c"], false, cx);
    cx.update(|window, cx| window.click("list-row-b", cx));
    advance(cx, PANEL);
    assert_eq!(view.read_with(cx, |h, _| h.clicks.clone()), ["b"]);
    assert!(said(&view, cx).is_empty());

    // Dropped where it was: not a move, and not a click either.
    let from = bounds("list-row-a", cx).center();
    press(from, cx);
    pull(from, point(from.x, from.y + px(10.)), cx);
    release(point(from.x, from.y + px(10.)), cx);
    settle(cx);
    assert!(
        said(&view, cx).is_empty(),
        "a row dropped in its place says nothing"
    );
    assert_eq!(view.read_with(cx, |h, _| h.clicks.clone()), ["b"]);
    assert_eq!(top("a", cx), 10.);
}

#[gpui_kit::test]
fn the_dropped_row_settles_into_its_place(cx: &mut TestAppContext) {
    let (_, cx) = host(&[], &["a", "b", "c"], false, cx);
    let a = top("a", cx);
    let to = centre("c", cx) + 5.;
    let (_, held) = lift("list-row-a", to, cx);
    advance(cx, PANEL * 2);
    let held_at = top("a", cx);
    release(held, cx);
    // It is where it was let go, then eases to the slot.
    let at_drop = top("a", cx);
    assert!(
        (at_drop - held_at).abs() < 6.,
        "no jump at the drop: {at_drop} {held_at}"
    );
    advance(cx, PANEL / 3);
    let mid = top("a", cx);
    assert!(mid != at_drop && mid != a + 2. * PITCH, "on its way: {mid}");
    advance(cx, PANEL * 2);
    assert_eq!(top("a", cx), a + 2. * PITCH);
    assert_eq!(top("c", cx), a + PITCH);
}

#[gpui_kit::test]
fn a_refused_move_slides_everything_back(cx: &mut TestAppContext) {
    let (view, cx) = host(&[], &["a", "b", "c"], false, cx);
    view.update(cx, |h, _| h.accepts = false);
    let tops = ["a", "b", "c"].map(|k| top(k, cx));
    drag("list-row-a", centre("c", cx) + 5., cx);
    assert_eq!(said(&view, cx).len(), 1, "it asked");
    settle(cx);
    assert_eq!(["a", "b", "c"].map(|k| top(k, cx)), tops);
}

#[gpui_kit::test]
fn escape_cancels_the_drag_and_the_row_slides_back(cx: &mut TestAppContext) {
    let (view, cx) = host(&[], &["a", "b", "c"], false, cx);
    let tops = ["a", "b", "c"].map(|k| top(k, cx));
    let (_, held) = lift("list-row-a", centre("c", cx) + 5., cx);
    advance(cx, PANEL);
    assert_ne!(top("b", cx), tops[1], "the others made way");
    cx.simulate_keystrokes("escape");
    advance(cx, PANEL * 2);
    assert_eq!(["a", "b", "c"].map(|k| top(k, cx)), tops);
    assert!(!shown("list-slot", cx));
    // The button comes up afterwards: it moves nothing.
    release(held, cx);
    settle(cx);
    assert!(said(&view, cx).is_empty(), "a cancelled drag says nothing");
    assert_eq!(["a", "b", "c"].map(|k| top(k, cx)), tops);
}

#[gpui_kit::test]
fn a_drop_outside_the_list_cancels(cx: &mut TestAppContext) {
    let (view, cx) = host(&[], &["a", "b", "c"], false, cx);
    let tops = ["a", "b", "c"].map(|k| top(k, cx));
    let from = bounds("list-row-a", cx).center();
    press(from, cx);
    let away = point(px(700.), from.y + px(20.));
    pull(from, away, cx);
    release(away, cx);
    settle(cx);
    assert!(said(&view, cx).is_empty());
    assert_eq!(["a", "b", "c"].map(|k| top(k, cx)), tops);
}

#[gpui_kit::test]
fn a_row_is_dragged_into_a_group_and_out_of_it(cx: &mut TestAppContext) {
    let (view, cx) = host(&[("g", true, &["x", "y"])], &["a", "b"], false, cx);
    // Between x and y.
    let between =
        f32::from(bounds("list-row-x", cx).bottom() + bounds("list-row-y", cx).top()) / 2.;
    drag("list-row-a", between, cx);
    assert_eq!(said(&view, cx), ["row a -> g @1"]);
    settle(cx);
    let tops = ["x", "a", "y", "b"].map(|k| top(k, cx));
    assert!(
        tops.windows(2).all(|w| w[0] < w[1]),
        "x, a, y, b in order: {tops:?}"
    );
    assert_eq!(top("a", cx) - top("x", cx), PITCH);

    // Out again, below the group.
    let below = centre("b", cx) + 5.;
    drag("list-row-x", below, cx);
    assert_eq!(said(&view, cx)[1], "row x -> - @1");
    settle(cx);
    let tops = ["a", "y", "b", "x"].map(|k| top(k, cx));
    assert!(
        tops.windows(2).all(|w| w[0] < w[1]),
        "a, y, b, x in order: {tops:?}"
    );
}

#[gpui_kit::test]
fn the_first_place_of_a_group_is_its_header(cx: &mut TestAppContext) {
    let (view, cx) = host(&[("g", true, &["x"])], &["a"], false, cx);
    let header = bounds("list-group-g-header", cx).center().y;
    drag("list-row-a", f32::from(header), cx);
    assert_eq!(said(&view, cx), ["row a -> g @0"]);
}

#[gpui_kit::test]
fn a_row_dropped_on_a_folded_group_goes_to_its_end(cx: &mut TestAppContext) {
    let (view, cx) = host(&[("g", false, &["x"])], &["a"], false, cx);
    let header = bounds("list-group-g-header", cx).center().y;
    drag("list-row-a", f32::from(header), cx);
    assert_eq!(said(&view, cx), ["row a -> g @1"]);
}

#[gpui_kit::test]
fn a_group_is_dragged_among_the_groups(cx: &mut TestAppContext) {
    let (view, cx) = host(
        &[
            ("g1", true, &["x"]),
            ("g2", true, &["y"]),
            ("g3", false, &[]),
        ],
        &["a"],
        false,
        cx,
    );
    let g1 = bounds("list-group-g1-header", cx).top();
    let g2_end = bounds("list-row-y", cx).bottom();
    let held = bounds("list-group-g1-header", cx).center();
    // The group is moved by its header, and goes by its middle: that is what is put below g2.
    let middle = bounds("list-group-g1", cx).center().y;
    press(held, cx);
    let to = point(held.x, held.y + (g2_end - middle));
    pull(held, to, cx);
    advance(cx, PANEL * 2);
    // g2 made way above it.
    assert_eq!(bounds("list-group-g2-header", cx).top(), g1);
    release(to, cx);
    assert_eq!(said(&view, cx), ["group g1 @1"]);
    settle(cx);
    assert!(bounds("list-group-g2-header", cx).top() < bounds("list-group-g1-header", cx).top());
    // The loose row stays below the groups.
    assert!(top("a", cx) > f32::from(bounds("list-group-g3-header", cx).bottom()) - 1.);
}

#[gpui_kit::test]
fn a_group_is_not_dragged_below_the_rows_outside_any_group(cx: &mut TestAppContext) {
    let (view, cx) = host(
        &[("g1", true, &["x"]), ("g2", true, &["y"])],
        &["a", "b"],
        false,
        cx,
    );
    let held = bounds("list-group-g1-header", cx).center();
    press(held, cx);
    let to = point(held.x, bounds("list-row-b", cx).bottom() + px(100.));
    pull(held, to, cx);
    release(to, cx);
    assert_eq!(said(&view, cx), ["group g1 @1"]);
}

#[gpui_kit::test]
fn a_group_folds_and_unfolds_with_its_count_and_its_mark(cx: &mut TestAppContext) {
    let (view, cx) = host(&[("g", true, &["x", "y"])], &["a"], false, cx);
    assert!(!shown("mark-g", cx), "no mark while it is open");
    let a = top("a", cx);
    let open_height = bounds("list-group-g-body", cx).size.height;
    assert_eq!(value("list-group-g-chevron"), Some(0.0));

    cx.update(|window, cx| window.click("list-group-g-header", cx));
    assert_eq!(said(&view, cx), ["toggle g false"]);
    advance(cx, PANEL / 3);
    let mid = bounds("list-group-g-body", cx).size.height;
    assert!(px(0.) < mid && mid < open_height, "folding: {mid:?}");
    let turned = value("list-group-g-chevron").unwrap();
    assert!(0.0 < turned && turned < 1.0, "the chevron turns: {turned}");
    assert!(top("a", cx) < a, "what is below follows");
    advance(cx, PANEL * 2);
    assert!(!shown("list-group-g-body", cx));
    assert_eq!(value("list-group-g-chevron"), Some(1.0));
    assert!(shown("mark-g", cx), "a folded group shows its mark");
    assert_eq!(pending_frames(cx), 0);

    cx.update(|window, cx| window.click("list-group-g-header", cx));
    assert_eq!(said(&view, cx)[1], "toggle g true");
    advance(cx, PANEL * 2);
    assert_eq!(top("a", cx), a);
    assert!(!shown("mark-g", cx));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_group_says_how_many_rows_it_holds(cx: &mut TestAppContext) {
    let (_, cx) = host(&[("g", true, &["x", "y"])], &[], false, cx);
    let label = cx.update(|window, _| window.find("list-group-g-count").label().map(str::to_owned));
    assert_eq!(label.as_deref(), Some("2"));
}

#[gpui_kit::test]
fn with_reduced_motion_the_rows_jump(cx: &mut TestAppContext) {
    let (view, cx) = host(&[], &["a", "b", "c"], true, cx);
    let (b, a) = (top("b", cx), top("a", cx));
    let from = bounds("list-row-a", cx).center();
    press(from, cx);
    let to = point(from.x, px(centre("c", cx) + 5.));
    cx.simulate_mouse_move(to, Some(MouseButton::Left), NONE);
    advance(cx, FRAME);
    assert_eq!(
        top("b", cx),
        b - PITCH,
        "b is at its place in the first frame"
    );
    release(to, cx);
    advance(cx, FRAME);
    assert_eq!(said(&view, cx), ["row a -> - @2"]);
    assert_eq!(top("a", cx), a + 2. * PITCH, "settled at once");
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_press_on_a_row_lifts_it_only_once_it_moved(cx: &mut TestAppContext) {
    let (_, cx) = host(&[], &["a", "b"], false, cx);
    let from = bounds("list-row-a", cx).center();
    press(from, cx);
    advance(cx, PANEL);
    assert!(!shown("list-slot", cx), "a press alone starts nothing");
    cx.simulate_mouse_move(
        point(from.x, from.y + px(1.)),
        Some(MouseButton::Left),
        NONE,
    );
    advance(cx, PANEL);
    assert!(!shown("list-slot", cx), "a tremor starts nothing");
    cx.simulate_mouse_move(
        point(from.x, from.y + px(12.)),
        Some(MouseButton::Left),
        NONE,
    );
    advance(cx, HOVER);
    assert!(shown("list-slot", cx), "past the threshold it is lifted");
    release(point(from.x, from.y + px(12.)), cx);
}

#[gpui_kit::test]
fn nothing_redraws_at_rest_before_and_after_a_drag(cx: &mut TestAppContext) {
    let (_, cx) = host(&[("g", true, &["x"])], &["a", "b"], false, cx);
    assert_eq!(pending_frames(cx), 0);
    drag("list-row-a", centre("x", cx) + 5., cx);
    settle(cx);
    assert_eq!(pending_frames(cx), 0);
}
