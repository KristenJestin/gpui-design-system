//! What `changing` draws when its content changes (a mark or a word fading back in from the old
//! to the new) is noted in `seen` under the element's id, so a test sees the frames between.

mod common;

use common::{advance, init};
use gpui_design_system::motion::{HOVER, SWAP_FROM};
use gpui_design_system::{changing, seen::seen};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, InteractiveElement as _, IntoElement, ParentElement, Render, SharedString,
    TestAppContext, TestSupportExt as _, VisualTestContext, Window, div,
};

/// A mark, said by its name, that the view changes.
struct Marked {
    mark: SharedString,
}

impl Render for Marked {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        changing("marked", self.mark.clone(), |mark| {
            div()
                .id("marked-mark")
                .child(mark)
                .test_support()
                .into_any_element()
        })
    }
}

fn change_to(mark: &'static str, view: &gpui_kit::Entity<Marked>, cx: &mut VisualTestContext) {
    cx.update(|_, cx| {
        view.update(cx, |marked, cx| {
            marked.mark = mark.into();
            cx.notify()
        })
    });
    cx.update(|window, cx| window.render_frame(cx));
}

fn marked(cx: &mut TestAppContext) -> (gpui_kit::Entity<Marked>, &mut VisualTestContext) {
    init(cx);
    let (view, cx) = cx.add_window_view(|_, _| Marked {
        mark: "check".into(),
    });
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

#[gpui_kit::test]
fn a_changed_mark_is_seen_partway_between_0_and_1_and_at_rest_at_1(cx: &mut TestAppContext) {
    let (view, cx) = marked(cx);
    assert_eq!(seen("marked"), Some(1.0), "a first mark is simply there");

    change_to("cross", &view, cx);
    let at_once = seen("marked").expect("a changed mark is noted");
    assert!(
        (at_once - SWAP_FROM).abs() < 0.01,
        "it starts from SWAP_FROM: {at_once}"
    );

    advance(cx, HOVER / 2);
    let partway = seen("marked").expect("noted partway");
    assert!(
        0.0 < partway && partway < 1.0,
        "strictly between 0 and 1 partway: {partway}"
    );
    assert!(partway > at_once, "and coming back in: {partway}");

    advance(cx, HOVER * 2);
    assert_eq!(seen("marked"), Some(1.0), "at rest at 1");
}

#[gpui_kit::test]
fn with_reduced_motion_a_changed_mark_jumps_to_1(cx: &mut TestAppContext) {
    let (view, cx) = marked(cx);
    cx.update(|_, cx| cx.set_reduce_motion(true));

    change_to("cross", &view, cx);
    assert_eq!(seen("marked"), Some(1.0), "it jumps, never seen partway");
    advance(cx, HOVER / 2);
    assert_eq!(seen("marked"), Some(1.0));
}
