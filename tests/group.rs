//! A `Group` (a header that folds its rows, which comes and goes with them) and `Grouped` (the
//! rows of several groups, one moving from a group to another), on GPUI's test platform.

mod common;

use std::time::Duration;

use common::{advance, init, pending_frames};
use gpui_design_system::motion::{HOVER, PANEL};
use gpui_design_system::{Group, Grouped, Item};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, Entity, InteractiveElement as _, IntoElement, ParentElement, Render, Styled,
    TestAppContext, TestSupportExt as _, VisualTestContext, Window, div, px,
};

const GROUPS: [(&str, &str, &str); 2] = [
    ("asked", "Asked", "Always a card for you."),
    ("judged", "Judged", "The judge decides."),
];
const ROW: f32 = 24.;

struct Host {
    grouped: Grouped<&'static str, &'static str>,
    /// Each row: its key and its group.
    rows: Vec<(&'static str, &'static str)>,
    folded: Vec<&'static str>,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = cx.background_executor().now();
        self.grouped.sync(
            GROUPS.map(|(group, ..)| group),
            self.rows
                .iter()
                .map(|(key, group)| (key.to_string(), *group, *key)),
            now,
            cx.reduce_motion(),
        );
        let this = cx.entity().downgrade();
        let mut column = div().w(px(400.)).flex().flex_col();
        for ((_, list), (group, title, sentence)) in self.grouped.groups().zip(GROUPS) {
            let this = this.clone();
            column = column.child(
                Group::new(group, title, list.len())
                    .sentence(sentence)
                    .open(!self.folded.contains(&group))
                    .on_toggle(move |open, _, cx| {
                        this.update(cx, |this, cx| {
                            this.folded.retain(|folded| *folded != group);
                            if !open {
                                this.folded.push(group);
                            }
                            cx.notify();
                        })
                        .ok();
                    })
                    .children(list.rows().map(|row| {
                        Item::new(
                            format!("{group}-{}", row.key),
                            row.placement,
                            div().h(px(ROW)).child(*row.value),
                        )
                        .into_any_element()
                    })),
            );
        }
        column.child(div().id("after").h(px(20.)).test_support())
    }
}

fn host<'a>(
    rows: &[(&'static str, &'static str)],
    reduce: bool,
    cx: &'a mut TestAppContext,
) -> (Entity<Host>, &'a mut VisualTestContext) {
    init(cx);
    cx.update(|cx| cx.set_reduce_motion(reduce));
    let rows = rows.to_vec();
    let (view, cx) = cx.add_window_view(move |_, _| Host {
        grouped: Grouped::default(),
        rows,
        folded: Vec::new(),
    });
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, PANEL * 2);
    (view, cx)
}

fn set(view: &Entity<Host>, rows: &[(&'static str, &'static str)], cx: &mut VisualTestContext) {
    let rows = rows.to_vec();
    view.update(cx, |this, cx| {
        this.rows = rows;
        cx.notify();
    });
    cx.update(|window, cx| window.render_frame(cx));
}

fn after(cx: &mut VisualTestContext) -> f32 {
    cx.update(|window, _| f32::from(window.find("after").bounds().origin.y))
}

fn shown(id: &str, cx: &mut VisualTestContext) -> bool {
    cx.update(|window, _| window.try_find(id.to_owned()).is_some())
}

fn value(id: &str) -> Option<f32> {
    gpui_design_system::seen::seen(id.to_owned())
}

#[gpui_kit::test]
fn a_group_header_says_its_name_its_count_and_its_sentence(cx: &mut TestAppContext) {
    let (_, cx) = host(&[("a", "judged"), ("b", "judged")], false, cx);
    cx.update(|window, _| {
        let header = window.find("judged");
        assert_eq!(header.expanded(), Some(true));
        assert_eq!(window.find("judged-title").label(), Some("Judged"));
        assert_eq!(window.find("judged-count").label(), Some("2"));
        assert_eq!(
            window.find("judged-sentence").label(),
            Some("The judge decides.")
        );
    });
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_group_header_folds_its_rows(cx: &mut TestAppContext) {
    let (view, cx) = host(&[("a", "judged"), ("b", "judged")], false, cx);
    let open = after(cx);
    assert_eq!(value("judged-chevron"), Some(0.0));

    cx.update(|window, cx| window.click("judged", cx));
    assert_eq!(
        view.read_with(cx, |this, _| this.folded.clone()),
        ["judged"]
    );
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, PANEL / 3);
    let part = after(cx);
    assert!(open - 2. * ROW < part && part < open, "folding: {part}");
    let turned = value("judged-chevron").unwrap();
    assert!(0.0 < turned && turned < 1.0, "turning: {turned}");
    advance(cx, PANEL * 2);
    assert_eq!(
        after(cx),
        open - 2. * ROW,
        "the header stays, its rows went"
    );
    assert_eq!(value("judged-chevron"), Some(1.0));
    assert!(!shown("judged-a", cx));
    assert_eq!(pending_frames(cx), 0);

    cx.update(|window, cx| window.click("judged", cx));
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, PANEL / 3);
    let unfolding = after(cx);
    assert!(
        open - 2. * ROW < unfolding && unfolding < open,
        "unfolding: {unfolding}"
    );
    advance(cx, PANEL * 2);
    assert_eq!(after(cx), open, "no jump at the end");
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn an_emptied_group_folds_away(cx: &mut TestAppContext) {
    let (view, cx) = host(&[("a", "asked"), ("b", "judged")], false, cx);
    let full = after(cx);
    assert!(shown("asked", cx));

    set(&view, &[("b", "judged")], cx);
    advance(cx, PANEL / 3);
    assert!(shown("asked", cx), "its header is still there, folding");
    assert_eq!(
        cx.update(|window, _| window
            .try_find("asked-count")
            .and_then(|count| count.label().map(str::to_owned))),
        Some("1".to_owned()),
        "it keeps saying what it held while it folds"
    );
    let part = after(cx);
    assert!(part < full, "folding away: {part}");
    advance(cx, PANEL * 2);
    assert!(!shown("asked", cx), "the header is gone");
    assert!(after(cx) < full - ROW, "and the room it took");
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_group_that_gets_its_first_row_unfolds(cx: &mut TestAppContext) {
    let (view, cx) = host(&[("b", "judged")], false, cx);
    let empty = after(cx);
    assert!(!shown("asked", cx), "an empty group has no header");

    set(&view, &[("a", "asked"), ("b", "judged")], cx);
    advance(cx, PANEL / 3);
    let part = after(cx);
    assert!(shown("asked", cx));
    assert!(empty < part, "unfolding in: {part}");
    advance(cx, PANEL * 2);
    let full = after(cx);
    assert!(full > part && full > empty + ROW, "unfolded: {full}");
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_row_that_changes_group_leaves_one_and_arrives_in_the_other(cx: &mut TestAppContext) {
    let (view, cx) = host(&[("a", "judged"), ("b", "judged")], false, cx);
    let before = after(cx);

    set(&view, &[("a", "asked"), ("b", "judged")], cx);
    let first = after(cx);
    assert!(
        (first - before).abs() < 2.,
        "nothing jumps when the row is moved: {first} for {before}"
    );
    assert!(shown("judged-a", cx), "it is still in the group it leaves");
    advance(cx, PANEL / 3);
    assert!(shown("judged-a", cx), "folding away there");
    assert!(shown("asked-a", cx), "unfolding here");
    advance(cx, PANEL * 2);
    assert!(!shown("judged-a", cx), "it left");
    assert!(shown("asked-a", cx), "it arrived");
    assert!(shown("judged-b", cx), "the other row stayed");
    cx.update(|window, _| assert_eq!(window.find("asked-count").label(), Some("1")));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_group_header_fades_its_hover(cx: &mut TestAppContext) {
    let (_, cx) = host(&[("a", "judged")], false, cx);
    // The pointer starts at the window's corner, on the header: it rests elsewhere first.
    cx.update(|window, cx| window.hover("after", cx));
    advance(cx, HOVER * 2);
    assert_eq!(value("judged"), Some(0.0));
    cx.update(|window, cx| window.hover("judged", cx));
    advance(cx, HOVER / 3);
    let part = value("judged").unwrap();
    assert!(0.0 < part && part < 1.0, "part way: {part}");
    advance(cx, HOVER * 2);
    assert_eq!(value("judged"), Some(1.0));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_group_jumps_with_reduced_motion(cx: &mut TestAppContext) {
    let (view, cx) = host(&[("b", "judged")], true, cx);
    set(&view, &[("a", "asked"), ("b", "judged")], cx);
    advance(cx, Duration::from_millis(16));
    assert!(shown("asked", cx), "the header is there at once");
    assert_eq!(pending_frames(cx), 0);
    set(&view, &[("b", "judged")], cx);
    advance(cx, Duration::from_millis(16));
    assert!(!shown("asked", cx), "and gone at once");
    assert_eq!(pending_frames(cx), 0);
}
