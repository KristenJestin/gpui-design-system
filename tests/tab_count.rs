//! `Tabs`: the count of a counted tab scales and fades in when it appears, scales and fades out
//! when it goes, and its number fades in again when it changes; each one stops asking for frames
//! once it has arrived, and with reduced motion each one jumps.

mod common;

use common::{advance, init, pending_frames};
use gpui_design_system::Tabs;
use gpui_design_system::motion::{HOVER, PANEL};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, IntoElement, ParentElement, Render, SharedString, Styled, TestAppContext,
    VisualTestContext, Window, div,
};

struct Counting {
    count: SharedString,
}

impl Render for Counting {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            Tabs::new("pages", "Pages", 0)
                .tab("one", "One")
                .counted("two", "Two", self.count.clone())
                .tab("three", "Three"),
        )
    }
}

fn counting<'a>(
    count: &'static str,
    reduce: bool,
    cx: &'a mut TestAppContext,
) -> (gpui_kit::Entity<Counting>, &'a mut VisualTestContext) {
    init(cx);
    cx.update(|cx| cx.set_reduce_motion(reduce));
    let (view, cx) = cx.add_window_view(move |_, _| Counting {
        count: count.into(),
    });
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, PANEL * 2);
    (view, cx)
}

fn set(view: &gpui_kit::Entity<Counting>, count: &'static str, cx: &mut VisualTestContext) {
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.count = count.into();
            cx.notify();
        })
    });
    cx.update(|window, cx| window.render_frame(cx));
}

fn width(id: &'static str, cx: &mut VisualTestContext) -> f32 {
    cx.update(|window, _| f32::from(window.find(id).bounds().size.width))
}

#[gpui_kit::test]
fn a_count_there_from_the_first_frame_is_simply_there(cx: &mut TestAppContext) {
    let (_, cx) = counting("3", false, cx);
    cx.update(|window, _| assert_eq!(window.find("pages-two-count").label(), Some("3")));
    assert_eq!(pending_frames(cx), 0, "nothing moves at rest");
}

#[gpui_kit::test]
fn an_empty_count_is_no_count(cx: &mut TestAppContext) {
    let (_, cx) = counting("", false, cx);
    cx.update(|window, _| assert!(window.try_find("pages-two-count").is_none()));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_count_that_appears_scales_and_fades_in_and_the_tab_widens_with_it(cx: &mut TestAppContext) {
    let (view, cx) = counting("", false, cx);
    let (tab_before, _) = (width("pages-two", cx), ());
    set(&view, "1", cx);
    assert!(pending_frames(cx) > 0, "the count is arriving");
    advance(cx, PANEL / 4);
    let (count_now, tab_now) = (width("pages-two-count", cx), width("pages-two", cx));
    assert!(count_now > 0., "it is there while it arrives");
    assert!(tab_now > tab_before, "the tab widens as it arrives");
    advance(cx, PANEL * 2);
    let (count_end, tab_end) = (width("pages-two-count", cx), width("pages-two", cx));
    assert!(
        count_now < count_end,
        "it was smaller on the way: {count_now} {count_end}"
    );
    assert!(
        tab_now < tab_end,
        "and the tab narrower: {tab_now} {tab_end}"
    );
    assert_eq!(pending_frames(cx), 0, "it stopped once it was there");
    cx.update(|window, _| assert_eq!(window.find("pages-two-count").label(), Some("1")));
}

#[gpui_kit::test]
fn a_count_that_goes_scales_and_fades_out_before_it_is_gone(cx: &mut TestAppContext) {
    let (view, cx) = counting("2", false, cx);
    let (count_before, tab_before) = (width("pages-two-count", cx), width("pages-two", cx));
    set(&view, "", cx);
    advance(cx, PANEL / 4);
    cx.update(|window, _| {
        assert_eq!(
            window.find("pages-two-count").label(),
            Some("2"),
            "it says what it said while it leaves"
        )
    });
    let (count_now, tab_now) = (width("pages-two-count", cx), width("pages-two", cx));
    assert!(
        count_now < count_before,
        "it is shrinking: {count_now} {count_before}"
    );
    assert!(
        tab_now < tab_before,
        "and the tab with it: {tab_now} {tab_before}"
    );
    advance(cx, PANEL * 2);
    cx.update(|window, _| {
        assert!(
            window.try_find("pages-two-count").is_none(),
            "then it is gone"
        )
    });
    assert!(width("pages-two", cx) < tab_now);
    assert_eq!(pending_frames(cx), 0, "it stopped once it was gone");
}

#[gpui_kit::test]
fn a_count_that_changes_says_its_new_number_and_fades_it_in(cx: &mut TestAppContext) {
    let (view, cx) = counting("3", false, cx);
    set(&view, "4", cx);
    cx.update(|window, _| assert_eq!(window.find("pages-two-count").label(), Some("4")));
    assert!(pending_frames(cx) > 0, "the number is fading in");
    advance(cx, HOVER * 2);
    assert_eq!(pending_frames(cx), 0, "it stopped once it was there");
}

#[gpui_kit::test]
fn with_reduced_motion_a_count_jumps_in_and_out(cx: &mut TestAppContext) {
    let (view, cx) = counting("", true, cx);
    set(&view, "1", cx);
    cx.update(|window, _| assert_eq!(window.find("pages-two-count").label(), Some("1")));
    let full = width("pages-two", cx);
    assert_eq!(pending_frames(cx), 0, "no frame for it");
    set(&view, "", cx);
    cx.update(|window, _| assert!(window.try_find("pages-two-count").is_none()));
    assert!(width("pages-two", cx) < full);
    assert_eq!(pending_frames(cx), 0);
}
