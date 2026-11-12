//! One page of several at a time: `sliding` slides the page shown in from the side of the move
//! (the next from the right, the previous from the left) while it fades in, and `Dots` says, a
//! dot per page, which is shown and which are done, each change fading.

mod common;

use common::{advance, init, pending_frames};
use gpui_design_system::motion::{HOVER, PANEL};
use gpui_design_system::{Dots, sliding};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, Entity, InteractiveElement as _, IntoElement, ParentElement, Render, Styled,
    TestAppContext, TestSupportExt as _, VisualTestContext, Window, div, px,
};

struct Host {
    page: usize,
    done: Vec<bool>,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut dots = Dots::new("steps", "Pages").current(self.page);
        for done in &self.done {
            dots = dots.dot(*done);
        }
        div()
            .w(px(600.))
            .p(px(16.))
            .flex()
            .flex_col()
            .child(dots)
            .child(sliding(
                "pages",
                self.page,
                div()
                    .id(format!("page-{}", self.page))
                    .h(px(40.))
                    .child(format!("Page {}", self.page + 1))
                    .test_support(),
            ))
    }
}

fn host(reduce: bool, cx: &mut TestAppContext) -> (Entity<Host>, &mut VisualTestContext) {
    init(cx);
    cx.update(|cx| cx.set_reduce_motion(reduce));
    let (view, cx) = cx.add_window_view(|_, _| Host {
        page: 0,
        done: vec![false; 3],
    });
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

fn x_of(id: &str, cx: &mut VisualTestContext) -> f32 {
    let id = id.to_owned();
    cx.update(|window, _| f32::from(window.find(id).bounds().origin.x))
}

fn width_of(id: &str, cx: &mut VisualTestContext) -> f32 {
    let id = id.to_owned();
    cx.update(|window, _| f32::from(window.find(id).bounds().size.width))
}

fn go(view: &Entity<Host>, page: usize, cx: &mut VisualTestContext) {
    view.update(cx, |host, cx| {
        host.page = page;
        cx.notify();
    });
    cx.update(|window, cx| window.render_frame(cx));
}

fn seen(id: &str) -> Option<f32> {
    gpui_design_system::seen::seen(id.to_owned())
}

#[gpui_kit::test]
fn the_next_page_slides_in_from_the_right_and_the_previous_from_the_left(cx: &mut TestAppContext) {
    let (view, cx) = host(false, cx);
    let rest = x_of("page-0", cx);
    assert_eq!(seen("pages"), Some(1.0), "the first page is simply there");

    go(&view, 1, cx);
    advance(cx, PANEL / 3);
    let moving = x_of("page-1", cx);
    assert!(moving > rest, "from the right: {moving} > {rest}");
    let shown = seen("pages").unwrap();
    assert!(0.0 < shown && shown < 1.0, "fading in: {shown}");
    advance(cx, PANEL * 2);
    assert_eq!(x_of("page-1", cx), rest);
    assert_eq!(seen("pages"), Some(1.0));
    assert_eq!(pending_frames(cx), 0, "nothing moves at rest");

    go(&view, 0, cx);
    advance(cx, PANEL / 3);
    let moving = x_of("page-0", cx);
    assert!(moving < rest, "from the left: {moving} < {rest}");
    advance(cx, PANEL * 2);
    assert_eq!(x_of("page-0", cx), rest);
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_page_drawn_again_at_the_same_index_does_not_slide(cx: &mut TestAppContext) {
    let (view, cx) = host(false, cx);
    go(&view, 0, cx);
    assert_eq!(seen("pages"), Some(1.0));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn with_reduced_motion_a_page_jumps(cx: &mut TestAppContext) {
    let (view, cx) = host(true, cx);
    let rest = x_of("page-0", cx);
    go(&view, 1, cx);
    assert_eq!(x_of("page-1", cx), rest);
    assert_eq!(seen("pages"), Some(1.0));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_dot_per_page_says_which_is_shown_and_which_are_done(cx: &mut TestAppContext) {
    let (_, cx) = host(false, cx);
    cx.update(|window, _| {
        let steps = window.find("steps");
        assert_eq!(steps.label(), Some("Pages"));
        assert_eq!(window.find("steps-0").label(), Some("1 of 3"));
        assert_eq!(window.find("steps-0").selected(), Some(true));
        assert_eq!(window.find("steps-1").selected(), Some(false));
        assert!(window.try_find("steps-3").is_none());
    });
    assert!(
        width_of("steps-0", cx) > width_of("steps-1", cx),
        "the dot of the page shown is longer"
    );
}

#[gpui_kit::test]
fn a_dot_done_fades_and_the_mark_of_the_page_shown_moves(cx: &mut TestAppContext) {
    let (view, cx) = host(false, cx);
    let short = width_of("steps-1", cx);
    let long = width_of("steps-0", cx);
    assert_eq!(seen("steps-0-done"), Some(0.0));

    view.update(cx, |host, cx| {
        host.done[0] = true;
        host.page = 1;
        cx.notify();
    });
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, HOVER / 3);
    let done = seen("steps-0-done").unwrap();
    assert!(0.0 < done && done < 1.0, "fading: {done}");
    let growing = width_of("steps-1", cx);
    assert!(short < growing && growing < long, "growing: {growing}");
    advance(cx, PANEL * 2);
    assert_eq!(seen("steps-0-done"), Some(1.0));
    assert_eq!(width_of("steps-1", cx), long);
    assert_eq!(width_of("steps-0", cx), short);
    cx.update(|window, _| {
        assert_eq!(window.find("steps-0").label(), Some("1 of 3, done"));
        assert_eq!(window.find("steps-1").selected(), Some(true));
    });
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn with_reduced_motion_the_dots_jump(cx: &mut TestAppContext) {
    let (view, cx) = host(true, cx);
    let long = width_of("steps-0", cx);
    view.update(cx, |host, cx| {
        host.done[0] = true;
        host.page = 1;
        cx.notify();
    });
    cx.update(|window, cx| window.render_frame(cx));
    assert_eq!(seen("steps-0-done"), Some(1.0));
    assert_eq!(width_of("steps-1", cx), long);
    assert_eq!(pending_frames(cx), 0);
}
