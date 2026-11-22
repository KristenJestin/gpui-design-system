//! The label of a picker's button that changes fades back in over `HOVER`, as every state that
//! changes moves, on GPUI's test platform.

mod common;

use common::{advance, init, pending_frames};
use gpui_design_system::Dropdown;
use gpui_design_system::motion::HOVER;
use gpui_design_system::seen::seen;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div, px, size,
};

struct Host {
    label: &'static str,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .p(px(16.))
            .child(Dropdown::new("picker", self.label).on_toggle(|_, _| {}))
    }
}

#[gpui_kit::test]
fn a_dropdown_label_that_changes_fades_in_over_hover(cx: &mut TestAppContext) {
    init(cx);
    let (view, cx) = cx.add_window_view(|_, _| Host { label: "One" });
    cx.simulate_resize(size(px(600.), px(400.)));
    cx.update(|window, cx| window.render_frame(cx));
    assert_eq!(
        seen("picker-label"),
        Some(1.0),
        "a first label is simply there"
    );
    view.update(cx, |host, cx| {
        host.label = "Two";
        cx.notify();
    });
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, HOVER / 3);
    let midway = seen("picker-label").unwrap();
    assert!(midway < 1.0, "the new label is fading in: {midway}");
    advance(cx, HOVER * 2);
    assert_eq!(seen("picker-label"), Some(1.0));
    assert_eq!(pending_frames(cx), 0);
}
