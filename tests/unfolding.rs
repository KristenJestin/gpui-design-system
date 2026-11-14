//! An `Unfolding` row: a line that unfolds its content under it in place, its chevron turning,
//! on GPUI's test platform.

mod common;

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use common::{advance, init, pending_frames};
use gpui_design_system::motion::{HOVER, MODAL, PANEL};
use gpui_design_system::{Modal, Unfolding};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, Entity, InteractiveElement as _, IntoElement, ParentElement, Render, ScrollDelta,
    SharedString, StatefulInteractiveElement as _, Styled, TestAppContext, TestSupportExt as _,
    VisualTestContext, Window, div, point, px, size,
};

const SHORT: &str = "Open a URL in the active tab.";
const LONG: &str = "Evaluate a script in the page and return what it returned. The script runs \
    inside the page and can read anything the page can read, so a value that holds a secret is \
    returned whole to whoever asked for it.";

struct Host {
    open: bool,
    wide: bool,
    text: &'static str,
    pressed: Rc<Cell<usize>>,
}

/// Rows of the dialog above and below the one under test: more than the window holds, so that
/// the dialog's body scrolls.
fn filler(prefix: &'static str, count: usize) -> impl Iterator<Item = gpui_kit::AnyElement> {
    (0..count).map(move |at| {
        div()
            .id(SharedString::from(format!("{prefix}-{at}")))
            .h(px(30.))
            .child(format!("row {at}"))
            .test_support()
            .into_any_element()
    })
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        let pressed = self.pressed.clone();
        let modal = Modal::new("dialog", "Review")
            .open(true)
            .children(filler("above", 6))
            .child(
                Unfolding::new("row", self.open)
                    .leading(div().id("box").size(px(14.)).test_support())
                    .line("browser_open")
                    .trailing(
                        div()
                            .id("pill")
                            .w(px(60.))
                            .h(px(20.))
                            .child("Judged")
                            .on_click(move |_, _, _| pressed.set(pressed.get() + 1))
                            .test_support(),
                    )
                    .body(div().id("text").child(self.text).test_support())
                    .on_toggle(move |open, _, cx| {
                        this.update(cx, |this, cx| {
                            this.open = open;
                            cx.notify();
                        })
                        .ok();
                    }),
            )
            .child(div().id("after").h(px(20.)).test_support())
            .children(filler("below", 20))
            .action(div().id("dialog-action").child("Accept"));
        if self.wide { modal.wide() } else { modal }
    }
}

/// The dialog drawn once, in a window too small for its body, nothing advanced yet.
fn drawn<'a>(
    open: bool,
    text: &'static str,
    reduce: bool,
    cx: &'a mut TestAppContext,
) -> (Entity<Host>, &'a mut VisualTestContext) {
    init(cx);
    cx.update(|cx| cx.set_reduce_motion(reduce));
    let (view, cx) = cx.add_window_view(move |_, _| Host {
        open,
        wide: false,
        text,
        pressed: Rc::default(),
    });
    cx.simulate_resize(size(px(900.), px(600.)));
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

/// The dialog once it has come in, its body scrolled a little: the rows under test are in sight
/// and what is below them has room to move.
fn host<'a>(
    open: bool,
    text: &'static str,
    reduce: bool,
    cx: &'a mut TestAppContext,
) -> (Entity<Host>, &'a mut VisualTestContext) {
    let (view, cx) = drawn(open, text, reduce, cx);
    advance(cx, MODAL * 2);
    let top = |cx: &mut VisualTestContext| {
        cx.update(|window, _| f32::from(window.find("above-2").bounds().origin.y))
    };
    let unscrolled = top(cx);
    cx.update(|window, cx| {
        window.scroll("above-2", ScrollDelta::Pixels(point(px(0.), px(-60.))), cx)
    });
    cx.update(|window, cx| window.render_frame(cx));
    assert!(top(cx) < unscrolled, "the dialog's body is scrolled");
    (view, cx)
}

fn after(cx: &mut VisualTestContext) -> f32 {
    cx.update(|window, _| f32::from(window.find("after").bounds().origin.y))
}

fn text_height(cx: &mut VisualTestContext) -> f32 {
    cx.update(|window, _| f32::from(window.find("text").bounds().size.height))
}

fn value(id: &str) -> Option<f32> {
    gpui_design_system::seen::seen(id.to_owned())
}

fn redraw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.render_frame(cx));
}

#[gpui_kit::test]
fn a_row_unfolds_its_content_on_click(cx: &mut TestAppContext) {
    let (view, cx) = host(false, SHORT, false, cx);
    cx.update(|window, _| {
        assert_eq!(window.find("row").expanded(), Some(false));
        assert!(window.try_find("text").is_none(), "no content while folded");
    });
    let closed = after(cx);

    cx.update(|window, cx| window.click("row", cx));
    assert!(view.read_with(cx, |this, _| this.open));
    redraw(cx);
    advance(cx, PANEL / 3);
    let part = after(cx);
    assert!(closed < part, "unfolding: {part}");
    advance(cx, PANEL * 2);
    let full = after(cx);
    assert!(full > part, "unfolded: {full}");
    cx.update(|window, _| assert_eq!(window.find("row").expanded(), Some(true)));
    assert_eq!(pending_frames(cx), 0, "at rest nothing is drawn");

    cx.update(|window, cx| window.click("row", cx));
    redraw(cx);
    advance(cx, PANEL / 3);
    let folding = after(cx);
    assert!(
        closed < folding && folding < full,
        "folding away: {folding}"
    );
    advance(cx, PANEL * 2);
    assert_eq!(after(cx), closed, "no jump at the end");
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn an_unfolding_row_turns_its_chevron(cx: &mut TestAppContext) {
    let (_, cx) = host(false, SHORT, false, cx);
    assert_eq!(value("row-chevron"), Some(1.0), "folded");
    cx.update(|window, cx| window.click("row", cx));
    redraw(cx);
    advance(cx, PANEL / 3);
    let turned = value("row-chevron").unwrap();
    assert!(0.0 < turned && turned < 1.0, "turning: {turned}");
    advance(cx, PANEL * 2);
    assert_eq!(value("row-chevron"), Some(0.0), "open");
    cx.update(|window, cx| window.click("row", cx));
    redraw(cx);
    advance(cx, PANEL * 2);
    assert_eq!(value("row-chevron"), Some(1.0), "folded again");
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn an_unfolded_row_measures_wrapped_lines(cx: &mut TestAppContext) {
    let (view, cx) = host(false, LONG, false, cx);
    let closed = after(cx);
    cx.update(|window, cx| window.click("row", cx));
    redraw(cx);
    advance(cx, PANEL * 2);
    let lines = text_height(cx);
    let wrapped = after(cx) - closed;
    assert!(
        wrapped >= lines && lines > 3. * 12.,
        "the content wraps onto several lines ({lines}) and the rows below make room for all of them ({wrapped})"
    );

    // The width changes while it is open: the lines wrap again and what is below follows.
    view.update(cx, |this, cx| {
        this.wide = true;
        cx.notify();
    });
    redraw(cx);
    let wide = after(cx);
    assert!(wide < closed + wrapped, "fewer lines: {wide}");
    assert!(text_height(cx) < lines);

    // And it folds from the height it has now, not from the one it had.
    cx.update(|window, cx| window.click("row", cx));
    redraw(cx);
    advance(cx, Duration::from_millis(16));
    let start = after(cx);
    assert!(
        closed < start && start <= wide,
        "folding from {wide}, not from the earlier height: {start}"
    );
    advance(cx, PANEL * 2);
    assert_eq!(after(cx), closed, "no jump at the end");
}

#[gpui_kit::test]
fn a_click_on_the_row_s_own_controls_does_not_toggle_it(cx: &mut TestAppContext) {
    let (view, cx) = host(false, SHORT, false, cx);
    cx.update(|window, cx| window.click("pill", cx));
    assert!(
        !view.read_with(cx, |this, _| this.open),
        "the pill is not the row"
    );
    assert_eq!(view.read_with(cx, |this, _| this.pressed.get()), 1);
    cx.update(|window, cx| window.click("box", cx));
    assert!(!view.read_with(cx, |this, _| this.open), "nor is the box");
}

/// How far under the row what follows it is: the dialog rising into place moves both alike.
fn room_below(cx: &mut VisualTestContext) -> f32 {
    cx.update(|window, _| {
        f32::from(window.find("after").bounds().origin.y - window.find("row").bounds().origin.y)
    })
}

#[gpui_kit::test]
fn an_unfolding_row_first_drawn_open_is_simply_open(cx: &mut TestAppContext) {
    let (_, cx) = drawn(true, SHORT, false, cx);
    cx.update(|window, _| assert!(window.try_find("text").is_some()));
    let (room, turned) = (room_below(cx), value("row-chevron"));
    assert_eq!(turned, Some(0.0));
    // The two frames after its first draw move nothing of it.
    for _ in 0..2 {
        advance(cx, Duration::from_millis(16));
        assert_eq!(room_below(cx), room, "the content did not unfold again");
        assert_eq!(value("row-chevron"), turned, "the chevron did not turn");
    }
    advance(cx, MODAL * 2);
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn an_unfolding_row_folded_back_mid_way_does_not_jump(cx: &mut TestAppContext) {
    let (_, cx) = host(false, SHORT, false, cx);
    let closed = after(cx);
    cx.update(|window, cx| window.click("row", cx));
    redraw(cx);
    advance(cx, PANEL / 2);
    let (opened, turned) = (after(cx), value("row-chevron").unwrap());
    assert!(closed < opened && 0. < turned && turned < 1., "mid-way");

    cx.update(|window, cx| window.click("row", cx));
    redraw(cx);
    let (now, turning) = (after(cx), value("row-chevron").unwrap());
    assert!(
        (now - opened).abs() < 2. && (turning - turned).abs() < 0.05,
        "nothing jumps when it turns back: {opened} / {now}, {turned} / {turning}"
    );
    let mut last = now;
    for _ in 0..(PANEL * 2).as_millis() / 16 {
        advance(cx, Duration::from_millis(16));
        let here = after(cx);
        assert!(
            here <= last && last - here < 12.,
            "it folds smoothly: {last} then {here}"
        );
        last = here;
    }
    assert_eq!(after(cx), closed, "no jump at the end");
    assert_eq!(pending_frames(cx), 0);

    // And the other way: unfolded again part way through its fold.
    cx.update(|window, cx| window.click("row", cx));
    redraw(cx);
    advance(cx, PANEL * 2);
    let full = after(cx);
    cx.update(|window, cx| window.click("row", cx));
    redraw(cx);
    advance(cx, PANEL / 2);
    let folding = after(cx);
    cx.update(|window, cx| window.click("row", cx));
    redraw(cx);
    assert!((after(cx) - folding).abs() < 2., "no jump");
    advance(cx, PANEL * 2);
    assert_eq!(after(cx), full);
}

#[gpui_kit::test]
fn an_unfolding_row_fades_its_hover(cx: &mut TestAppContext) {
    let (_, cx) = host(false, SHORT, false, cx);
    assert_eq!(value("row"), Some(0.0));
    cx.update(|window, cx| window.hover("row", cx));
    advance(cx, HOVER / 3);
    let part = value("row").unwrap();
    assert!(0.0 < part && part < 1.0, "part way: {part}");
    advance(cx, HOVER * 2);
    assert_eq!(value("row"), Some(1.0));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn an_unfolding_row_jumps_with_reduced_motion(cx: &mut TestAppContext) {
    let (_, cx) = host(false, SHORT, true, cx);
    let closed = after(cx);
    cx.update(|window, cx| window.click("row", cx));
    redraw(cx);
    advance(cx, Duration::from_millis(16));
    assert_eq!(value("row-chevron"), Some(0.0), "the chevron jumps");
    assert!(after(cx) > closed, "the content jumps open");
    assert_eq!(pending_frames(cx), 0);
}
