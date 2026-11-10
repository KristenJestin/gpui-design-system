//! `SetHere`: the mark of a value set at one level over the one it inherits: a dot, its words and
//! a way back, unfolding under the control while the value is set and folding away when it is not.

mod common;

use common::{advance, count_redraws, init, pending_frames};
use gpui_design_system::SetHere;
use gpui_design_system::motion::PANEL;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, Entity, InteractiveElement as _, IntoElement, ParentElement, Pixels, Render, Styled,
    TestAppContext, TestSupportExt as _, VisualTestContext, Window, div, px,
};

struct Row {
    set: bool,
    reset: usize,
}

impl Render for Row {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        div()
            .w(px(400.))
            .flex()
            .flex_col()
            .child(
                SetHere::new(
                    "lang",
                    div().id("control").h(px(30.)).test_support(),
                    "set here",
                    "Use the default",
                )
                .set(self.set)
                .on_reset(move |_, cx| {
                    this.update(cx, |this, cx| {
                        this.reset += 1;
                        this.set = false;
                        cx.notify();
                    })
                    .ok();
                }),
            )
            .child(div().id("after").h(px(20.)).test_support())
    }
}

fn row(set: bool, cx: &mut TestAppContext) -> (Entity<Row>, &mut VisualTestContext) {
    init(cx);
    let (view, cx) = cx.add_window_view(move |_, _| Row { set, reset: 0 });
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

fn top(id: &str, cx: &mut VisualTestContext) -> Pixels {
    let id = id.to_owned();
    cx.update(|window, _| window.find(id).bounds().origin.y)
}

fn toggle(view: &Entity<Row>, set: bool, cx: &mut VisualTestContext) {
    cx.update(|_, cx| {
        view.update(cx, |row, cx| {
            row.set = set;
            cx.notify();
        })
    });
    cx.update(|window, cx| window.render_frame(cx));
}

#[gpui_kit::test]
fn a_value_set_here_says_it_in_the_apps_words_and_offers_the_way_back(cx: &mut TestAppContext) {
    let (_, cx) = row(true, cx);
    cx.update(|window, _| {
        assert_eq!(window.find("lang-set").label(), Some("set here"));
        assert_eq!(window.find("lang-reset").label(), Some("Use the default"));
    });
    assert!(
        top("lang-set", cx) > top("control", cx),
        "under the control"
    );
}

#[gpui_kit::test]
fn a_value_that_is_not_set_here_draws_no_mark(cx: &mut TestAppContext) {
    let (_, cx) = row(false, cx);
    cx.update(|window, _| {
        assert!(window.try_find("lang-set").is_none());
        assert!(window.try_find("lang-reset").is_none());
        assert!(window.try_find("control").is_some(), "the control is there");
    });
}

#[gpui_kit::test]
fn the_way_back_tells_the_screen_and_the_mark_folds_away(cx: &mut TestAppContext) {
    let (view, cx) = row(true, cx);
    cx.update(|window, cx| window.click("lang-reset", cx));
    assert_eq!(view.read_with(cx, |row, _| row.reset), 1);
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, PANEL * 2);
    cx.update(|window, _| assert!(window.try_find("lang-set").is_none()));
}

#[gpui_kit::test]
fn the_mark_unfolds_under_the_control_and_pushes_what_follows_down_part_way(
    cx: &mut TestAppContext,
) {
    let (view, cx) = row(false, cx);
    let closed = top("after", cx);
    toggle(&view, true, cx);
    advance(cx, PANEL / 3);
    let part = top("after", cx);
    assert!(closed < part, "pushed down part way: {closed:?} < {part:?}");
    advance(cx, PANEL * 2);
    let open = top("after", cx);
    assert!(part < open, "and all the way: {part:?} < {open:?}");
    assert_eq!(pending_frames(cx), 0);

    toggle(&view, false, cx);
    advance(cx, PANEL / 3);
    let folding = top("after", cx);
    assert!(
        closed < folding && folding < open,
        "pulled back: {folding:?}"
    );
    advance(cx, PANEL * 2);
    assert_eq!(top("after", cx), closed, "no jump at the end");
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn with_reduced_motion_the_mark_comes_and_goes_at_once(cx: &mut TestAppContext) {
    let (view, cx) = row(false, cx);
    cx.update(|_, cx| cx.set_reduce_motion(true));
    let closed = top("after", cx);
    toggle(&view, true, cx);
    assert!(top("after", cx) > closed);
    assert_eq!(pending_frames(cx), 0);
    toggle(&view, false, cx);
    assert_eq!(top("after", cx), closed);
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_mark_at_rest_draws_nothing(cx: &mut TestAppContext) {
    let (view, cx) = row(true, cx);
    let redraws = count_redraws(&view, cx);
    advance(cx, PANEL * 2);
    assert_eq!(redraws.get(), 0);
    assert_eq!(pending_frames(cx), 0);
}
