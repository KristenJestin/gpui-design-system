//! A `Summary`: one line that stands for a set of groups and unfolds them under it, by a click or
//! when the screen says a search found something in it, on GPUI's test platform.

mod common;

use std::time::Duration;

use common::{advance, init, pending_frames};
use gpui_design_system::Summary;
use gpui_design_system::motion::{HOVER, PANEL};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, Entity, InteractiveElement as _, IntoElement, ParentElement, Render, Styled,
    TestAppContext, TestSupportExt as _, VisualTestContext, Window, div, px,
};

const BODY: f32 = 48.;

struct Host {
    open: bool,
    found: bool,
    toggled: Option<bool>,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        div()
            .w(px(500.))
            .flex()
            .flex_col()
            .child(
                Summary::new("rest", "31 unchanged", self.open)
                    .detail("3 allowed · 1 asked · 1 off · 26 judged")
                    .unfolded_by(self.found)
                    .on_toggle(move |open, _, cx| {
                        this.update(cx, |this, cx| {
                            this.open = open;
                            this.toggled = Some(open);
                            cx.notify();
                        })
                        .ok();
                    })
                    .child(
                        div()
                            .id("groups")
                            .h(px(BODY))
                            .child("The groups")
                            .test_support(),
                    ),
            )
            .child(div().id("after").h(px(20.)).test_support())
    }
}

fn host(reduce: bool, cx: &mut TestAppContext) -> (Entity<Host>, &mut VisualTestContext) {
    init(cx);
    cx.update(|cx| cx.set_reduce_motion(reduce));
    let (view, cx) = cx.add_window_view(|_, _| Host {
        open: false,
        found: false,
        toggled: None,
    });
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, PANEL * 2);
    (view, cx)
}

fn after(cx: &mut VisualTestContext) -> f32 {
    cx.update(|window, _| f32::from(window.find("after").bounds().origin.y))
}

fn value(id: &str) -> Option<f32> {
    gpui_design_system::seen::seen(id.to_owned())
}

fn found(view: &Entity<Host>, found: bool, cx: &mut VisualTestContext) {
    view.update(cx, |this, cx| {
        this.found = found;
        cx.notify();
    });
    cx.update(|window, cx| window.render_frame(cx));
}

#[gpui_kit::test]
fn a_summary_row_says_its_line_and_its_detail(cx: &mut TestAppContext) {
    let (_, cx) = host(false, cx);
    cx.update(|window, _| {
        assert_eq!(window.find("rest").expanded(), Some(false));
        assert_eq!(window.find("rest-title").label(), Some("31 unchanged"));
        assert_eq!(
            window.find("rest-detail").label(),
            Some("3 allowed · 1 asked · 1 off · 26 judged")
        );
        assert!(window.try_find("groups").is_none(), "nothing below it yet");
    });
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_summary_row_unfolds_when_told(cx: &mut TestAppContext) {
    let (view, cx) = host(false, cx);
    let closed = after(cx);
    assert_eq!(value("rest-chevron"), Some(1.0));

    found(&view, true, cx);
    advance(cx, PANEL / 3);
    let part = after(cx);
    assert!(closed < part && part < closed + BODY, "unfolding: {part}");
    let turned = value("rest-chevron").unwrap();
    assert!(0.0 < turned && turned < 1.0, "turning: {turned}");
    advance(cx, PANEL * 2);
    assert_eq!(after(cx), closed + BODY + 8.);
    assert_eq!(value("rest-chevron"), Some(0.0));
    cx.update(|window, _| assert_eq!(window.find("rest").expanded(), Some(true)));
    assert_eq!(pending_frames(cx), 0);

    found(&view, false, cx);
    advance(cx, PANEL * 2);
    assert_eq!(
        after(cx),
        closed,
        "it folds back when nothing is found in it"
    );
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_summary_row_unfolds_on_click(cx: &mut TestAppContext) {
    let (view, cx) = host(false, cx);
    let closed = after(cx);
    cx.update(|window, cx| window.click("rest", cx));
    assert!(view.read_with(cx, |this, _| this.open));
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, PANEL * 2);
    assert!(after(cx) > closed + BODY);
    cx.update(|window, cx| window.click("rest", cx));
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, PANEL * 2);
    assert_eq!(after(cx), closed, "no jump at the end");
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_summary_row_fades_its_hover(cx: &mut TestAppContext) {
    let (_, cx) = host(false, cx);
    cx.update(|window, cx| window.hover("after", cx));
    advance(cx, HOVER * 2);
    assert_eq!(value("rest"), Some(0.0));
    cx.update(|window, cx| window.hover("rest", cx));
    advance(cx, HOVER / 3);
    let part = value("rest").unwrap();
    assert!(0.0 < part && part < 1.0, "part way: {part}");
    advance(cx, HOVER * 2);
    assert_eq!(value("rest"), Some(1.0));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_summary_row_jumps_with_reduced_motion(cx: &mut TestAppContext) {
    let (view, cx) = host(true, cx);
    let closed = after(cx);
    found(&view, true, cx);
    advance(cx, Duration::from_millis(16));
    assert_eq!(value("rest-chevron"), Some(0.0), "the chevron jumps");
    assert!(after(cx) > closed + BODY, "the groups jump open");
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_click_on_a_summary_that_looks_open_folds_it(cx: &mut TestAppContext) {
    let (view, cx) = host(false, cx);
    let closed = after(cx);
    found(&view, true, cx);
    advance(cx, PANEL * 2);
    assert!(after(cx) > closed + BODY, "unfolded by the search");

    cx.update(|window, cx| window.click("rest", cx));
    assert_eq!(
        view.read_with(cx, |this, _| (this.toggled, this.open)),
        (Some(false), false),
        "the screen is told it folds"
    );
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, PANEL / 3);
    let folding = after(cx);
    assert!(
        closed < folding && folding < closed + BODY + 8.,
        "folding: {folding}"
    );
    advance(cx, PANEL * 2);
    assert_eq!(
        after(cx),
        closed,
        "it stays folded while the search still finds"
    );
    assert_eq!(value("rest-chevron"), Some(1.0));
    cx.update(|window, _| assert_eq!(window.find("rest").expanded(), Some(false)));
    assert_eq!(pending_frames(cx), 0);

    // Once nothing is found, the next search unfolds it again.
    found(&view, false, cx);
    advance(cx, PANEL * 2);
    found(&view, true, cx);
    advance(cx, PANEL * 2);
    assert!(after(cx) > closed + BODY);
}
