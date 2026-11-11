//! A `Disclosure`: a quiet line with a chevron that turns, a summary that gives way while it is
//! open, a hover that fades, and a body that unfolds and folds away under it.

mod common;

use std::time::Duration;

use common::{advance, init, pending_frames};
use gpui_design_system::Disclosure;
use gpui_design_system::motion::{HOVER, PANEL};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, Entity, InteractiveElement as _, IntoElement, ParentElement, Render, Styled,
    TestAppContext, TestSupportExt as _, VisualTestContext, Window, div, px,
};

struct Host {
    open: bool,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        div()
            .w(px(600.))
            .p(px(16.))
            .child(
                Disclosure::new("notes", "Instructions for this check", self.open)
                    .summary("Look at the tests first")
                    .body(
                        div()
                            .id("notes-text")
                            .h(px(80.))
                            .child("The body")
                            .test_support(),
                    )
                    .on_toggle(move |open, _, cx| {
                        this.update(cx, |this, cx| {
                            this.open = open;
                            cx.notify();
                        })
                        .ok();
                    }),
            )
            .child(div().id("after").h(px(20.)).test_support())
    }
}

fn host(
    open: bool,
    reduce: bool,
    cx: &mut TestAppContext,
) -> (Entity<Host>, &mut VisualTestContext) {
    init(cx);
    cx.update(|cx| cx.set_reduce_motion(reduce));
    let (view, cx) = cx.add_window_view(move |_, _| Host { open });
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

/// How far something of the disclosure was drawn (0 to 1), `None` while it never was.
fn value(id: &str, _: &mut VisualTestContext) -> Option<f32> {
    gpui_design_system::seen::seen(id.to_owned())
}

/// How far the body pushes what follows it down: its height and the gap above it.
const BODY: f32 = 88.;

fn after(cx: &mut VisualTestContext) -> f32 {
    cx.update(|window, _| f32::from(window.find("after").bounds().origin.y))
}

#[gpui_kit::test]
fn a_disclosure_unfolds_its_body_and_turns_its_chevron(cx: &mut TestAppContext) {
    let (view, cx) = host(false, false, cx);
    cx.update(|window, _| {
        let line = window.find("notes");
        assert_eq!(line.label(), Some("Instructions for this check"));
        assert_eq!(line.expanded(), Some(false));
    });
    assert_eq!(value("notes-chevron", cx), Some(1.0), "folded");
    assert_eq!(
        value("notes-summary", cx),
        Some(1.0),
        "the summary is shown"
    );
    let closed = after(cx);
    cx.update(|window, _| {
        assert!(
            window.try_find("notes-text").is_none(),
            "no body while folded"
        )
    });

    cx.update(|window, cx| window.click("notes", cx));
    assert!(view.read_with(cx, |this, _| this.open));
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, PANEL / 3);
    let turned = value("notes-chevron", cx).unwrap();
    assert!(0.0 < turned && turned < 1.0, "turning: {turned}");
    let grown = after(cx);
    assert!(
        closed < grown && grown < closed + BODY,
        "unfolding: {grown}"
    );
    advance(cx, PANEL * 2);
    assert_eq!(value("notes-chevron", cx), Some(0.0), "open");
    assert_eq!(
        value("notes-summary", cx),
        Some(0.0),
        "the summary gave way"
    );
    assert_eq!(after(cx), closed + BODY);
    cx.update(|window, _| assert_eq!(window.find("notes").expanded(), Some(true)));
    assert_eq!(pending_frames(cx), 0);

    cx.update(|window, cx| window.click("notes", cx));
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, PANEL / 3);
    let folding = after(cx);
    assert!(
        closed < folding && folding < closed + BODY,
        "folding away: {folding}"
    );
    advance(cx, PANEL * 2);
    assert_eq!(after(cx), closed, "no jump at the end");
    assert_eq!(value("notes-chevron", cx), Some(1.0));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_disclosure_first_drawn_open_is_simply_open(cx: &mut TestAppContext) {
    let (_, cx) = host(true, false, cx);
    assert_eq!(value("notes-chevron", cx), Some(0.0));
    cx.update(|window, _| assert!(window.try_find("notes-text").is_some()));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_disclosure_fades_its_hover(cx: &mut TestAppContext) {
    let (_, cx) = host(false, false, cx);
    assert_eq!(value("notes", cx), Some(0.0));
    cx.update(|window, cx| window.hover("notes", cx));
    advance(cx, HOVER / 3);
    let part = value("notes", cx).unwrap();
    assert!(0.0 < part && part < 1.0, "part way: {part}");
    advance(cx, HOVER * 2);
    assert_eq!(value("notes", cx), Some(1.0));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_disclosure_jumps_with_reduced_motion(cx: &mut TestAppContext) {
    let (_, cx) = host(false, true, cx);
    cx.update(|window, cx| window.hover("notes", cx));
    advance(cx, Duration::from_millis(16));
    assert_eq!(value("notes", cx), Some(1.0), "the hover jumps");
    cx.update(|window, cx| window.click("notes", cx));
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, Duration::from_millis(16));
    assert_eq!(value("notes-chevron", cx), Some(0.0), "the chevron jumps");
    cx.update(|window, _| assert!(window.try_find("notes-text").is_some(), "the body jumps"));
    assert_eq!(pending_frames(cx), 0);
}
