//! The notice (`Toast::notice`): something told to the user rather than the result of what they
//! did, which can be answered from where it is, with several buttons and a click on its lines,
//! and taken away by its key when it is answered elsewhere. On GPUI's test platform.

mod common;

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use common::{advance, init, pending_frames};
use gpui_design_system::motion::{ENTER, PANEL};
use gpui_design_system::toast::{NOTICE_STAYS, withdraw};
use gpui_design_system::{ButtonKind, Toast, ToastStack, toast};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AppContext as _, Context, Entity, IntoElement, ParentElement, Pixels, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, point, px, size,
};

struct Host {
    stack: Entity<ToastStack>,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().relative().child(self.stack.clone())
    }
}

fn toasts(cx: &mut TestAppContext) -> &mut VisualTestContext {
    init(cx);
    let (_, cx) = cx.add_window_view(|_, cx| Host {
        stack: cx.new(ToastStack::new),
    });
    cx.simulate_resize(size(px(1000.), px(800.)));
    cx.update(|window, cx| window.render_frame(cx));
    cx
}

fn bounds(id: &str, cx: &mut VisualTestContext) -> gpui_kit::Bounds<Pixels> {
    let id = id.to_owned();
    cx.update(|window, _| window.find(id).bounds())
}

fn shown(id: &str, cx: &mut VisualTestContext) -> bool {
    let id = id.to_owned();
    cx.update(|window, cx| {
        window.render_frame(cx);
        window.try_find(id).is_some()
    })
}

fn request(allow: Rc<Cell<u32>>, deny: Rc<Cell<u32>>) -> Toast {
    Toast::notice("Request #3 waits for you")
        .detail("Read .env")
        .primary_action("Allow", move |_, _| allow.set(allow.get() + 1))
        .quiet_action("Deny", move |_, _| deny.set(deny.get() + 1))
}

#[gpui_kit::test]
fn a_notice_carries_two_buttons_and_each_runs_then_the_notice_leaves(cx: &mut TestAppContext) {
    let cx = toasts(cx);
    let (allowed, denied) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
    cx.update(|_, cx| toast(request(allowed.clone(), denied.clone()), cx));
    advance(cx, ENTER * 2);
    let (card, first, second) = (
        bounds("toast-1", cx),
        bounds("toast-1-action", cx),
        bounds("toast-1-action-2", cx),
    );
    for button in [first, second] {
        assert!(
            card.contains(&button.origin) && card.contains(&button.bottom_right()),
            "{button:?} sits inside {card:?}"
        );
    }
    assert!(first.right() <= second.left(), "side by side, in order");
    assert_eq!((allowed.get(), denied.get()), (0, 0), "nothing runs alone");
    cx.update(|window, cx| window.click("toast-1-action", cx));
    assert_eq!((allowed.get(), denied.get()), (1, 0), "Allow ran, only it");
    advance(cx, PANEL * 2);
    assert!(!shown("toast-1", cx), "the notice leaves once answered");

    cx.update(|_, cx| toast(request(allowed.clone(), denied.clone()), cx));
    advance(cx, ENTER * 2);
    cx.update(|window, cx| window.click("toast-2-action-2", cx));
    assert_eq!((allowed.get(), denied.get()), (1, 1), "Deny ran, only it");
    advance(cx, PANEL * 2);
    assert!(!shown("toast-2", cx), "the notice leaves once answered");
    assert_eq!(pending_frames(cx), 0);
}

#[test]
fn a_notice_offers_its_buttons_in_the_kind_it_names() {
    let toast = request(Rc::default(), Rc::default()).action("Later", |_, _| {});
    assert_eq!(
        toast.buttons().collect::<Vec<_>>(),
        [
            ("Allow", ButtonKind::Primary),
            ("Deny", ButtonKind::Quiet),
            ("Later", ButtonKind::Secondary),
        ],
        "the one main answer is primary, the refusal quiet, a plain action secondary"
    );
}

#[gpui_kit::test]
fn a_click_on_the_lines_of_a_notice_opens_it_and_the_notice_leaves(cx: &mut TestAppContext) {
    let cx = toasts(cx);
    let opened = Rc::new(Cell::new(0));
    let ran = opened.clone();
    cx.update(|_, cx| {
        toast(
            Toast::notice("Failed")
                .detail("The build is red")
                .on_open(move |_, _| ran.set(ran.get() + 1)),
            cx,
        )
    });
    advance(cx, ENTER * 2);
    assert_eq!(opened.get(), 0);
    cx.update(|window, cx| window.click("toast-1-open", cx));
    assert_eq!(opened.get(), 1);
    advance(cx, PANEL * 2);
    assert!(!shown("toast-1", cx));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_notice_stays_while_it_is_hovered_and_leaves_after_its_time(cx: &mut TestAppContext) {
    let cx = toasts(cx);
    cx.update(|_, cx| toast(Toast::notice("Reported").detail("Done"), cx));
    advance(cx, ENTER * 2);
    advance(cx, Duration::from_secs(6));
    assert!(
        shown("toast-1", cx),
        "it stays longer than a success's four seconds"
    );
    cx.update(|window, cx| window.hover("toast-1", cx));
    advance(cx, NOTICE_STAYS * 2);
    assert!(shown("toast-1", cx), "it waits while the pointer is on it");
    cx.simulate_mouse_move(point(px(10.), px(10.)), None, Default::default());
    advance(cx, Duration::from_secs(2));
    advance(cx, PANEL * 2);
    assert!(
        !shown("toast-1", cx),
        "and leaves once the pointer has left"
    );
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_notice_withdrawn_by_its_key_folds_away_and_the_rest_waits(cx: &mut TestAppContext) {
    let cx = toasts(cx);
    cx.update(|_, cx| {
        toast(
            Toast::notice("Request #3 waits for you").keyed("notification-3"),
            cx,
        );
        toast(
            Toast::notice("Request #4 waits for you").keyed("notification-4"),
            cx,
        );
    });
    advance(cx, ENTER * 2);
    assert!(shown("toast-1", cx) && shown("toast-2", cx));
    cx.update(|_, cx| withdraw("notification-3", cx));
    advance(cx, PANEL * 2);
    assert!(!shown("toast-1", cx), "the withdrawn one folded away");
    assert!(shown("toast-2", cx), "the other stays");
    // A key that is not on screen (it left already) is no error.
    cx.update(|_, cx| withdraw("notification-3", cx));
    assert_eq!(pending_frames(cx), 0);
}
