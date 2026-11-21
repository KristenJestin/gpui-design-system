//! `Swatches`: a few colours to choose among, each a round button of its own colour with its name
//! as its label, the chosen one marked by a ring that moves in over `PANEL`.

mod common;

use std::cell::Cell;
use std::rc::Rc;

use common::{advance, init, pending_frames};
use gpui_design_system::motion::PANEL;
use gpui_design_system::{Swatch, Swatches};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AnyElement, AppContext as _, Context, Entity, IntoElement, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, hsla, px, size,
};

type Draw = Box<dyn Fn(&mut Window, &mut Context<Host>) -> AnyElement>;

struct Host {
    draw: Draw,
}

impl Render for Host {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child((self.draw)(window, cx))
    }
}

fn host(
    cx: &mut TestAppContext,
    draw: impl Fn(&mut Window, &mut Context<Host>) -> AnyElement + 'static,
) -> (Entity<Host>, &mut VisualTestContext) {
    init(cx);
    let (view, cx) = cx.add_window_view(move |_, _| Host {
        draw: Box::new(draw),
    });
    cx.simulate_resize(size(px(1000.), px(400.)));
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

fn redraw(view: &Entity<Host>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        view.update(cx, |_, cx| cx.notify());
        window.render_frame(cx);
    });
}

fn three(chosen: Rc<Cell<usize>>) -> impl Fn(&mut Window, &mut Context<Host>) -> AnyElement {
    move |_, cx| {
        let (seen, this) = (chosen.clone(), cx.entity().downgrade());
        Swatches::new("colour", "Colour")
            .swatch(Swatch::new("red", "Red", hsla(0., 0.7, 0.5, 1.)))
            .swatch(Swatch::new("green", "Green", hsla(0.33, 0.7, 0.4, 1.)))
            .swatch(Swatch::new("blue", "Blue", hsla(0.6, 0.7, 0.5, 1.)))
            .chosen(chosen.get())
            .on_choose(move |at, _, cx| {
                seen.set(at);
                this.update(cx, |_, cx| cx.notify()).ok();
            })
            .into_any_element()
    }
}

#[gpui_kit::test]
fn each_swatch_is_named_and_only_the_chosen_one_is_selected(cx: &mut TestAppContext) {
    let chosen = Rc::new(Cell::new(1));
    let (_, cx) = host(cx, three(chosen));
    cx.update(|window, _| {
        assert_eq!(window.find("colour-red").label(), Some("Red"));
        assert_eq!(window.find("colour-red").selected(), Some(false));
        assert_eq!(window.find("colour-green").selected(), Some(true));
        assert_eq!(window.find("colour-blue").selected(), Some(false));
    });
}

#[gpui_kit::test]
fn clicking_a_swatch_chooses_it_and_the_chosen_one_chooses_nothing(cx: &mut TestAppContext) {
    let chosen = Rc::new(Cell::new(1));
    let (view, cx) = host(cx, three(chosen.clone()));
    cx.update(|window, cx| window.click("colour-green", cx));
    assert_eq!(chosen.get(), 1, "the chosen one asks nothing");
    cx.update(|window, cx| window.click("colour-blue", cx));
    assert_eq!(chosen.get(), 2);
    redraw(&view, cx);
    advance(cx, PANEL * 2);
    cx.update(|window, _| {
        assert_eq!(window.find("colour-blue").selected(), Some(true));
        assert_eq!(window.find("colour-green").selected(), Some(false));
    });
}

#[gpui_kit::test]
fn the_ring_moves_in_and_nothing_is_drawn_once_it_arrived(cx: &mut TestAppContext) {
    let chosen = Rc::new(Cell::new(0));
    let (view, cx) = host(cx, three(chosen));
    cx.update(|window, cx| window.click("colour-blue", cx));
    redraw(&view, cx);
    assert!(
        pending_frames(cx) > 0,
        "the ring is moving, not jumping to the swatch"
    );
    advance(cx, PANEL * 2);
    assert_eq!(pending_frames(cx), 0);
}
