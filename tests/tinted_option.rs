//! An option of a picker, a select or a set of radio choices can lead with an icon in a colour its
//! owner gives, and a list's entry can carry a check mark that moves in and out over `PANEL` as the
//! choice changes, on GPUI's test platform. Callers that give none are drawn as before.

mod common;

use common::{advance, init, pending_frames};
use gpui_design_system::motion::{ENTER, HOVER, PANEL};
use gpui_design_system::seen::{colour, seen};
use gpui_design_system::{Dropdown, Icon, MenuItem, RadioChoices, Select, SelectOption, palette};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, Entity, Hsla, IntoElement, ParentElement, Render, Styled, TestAppContext,
    VisualTestContext, Window, div, px, size,
};

const TEAL: Hsla = Hsla {
    h: 0.47,
    s: 0.6,
    l: 0.5,
    a: 1.,
};
const AMBER: Hsla = Hsla {
    h: 0.11,
    s: 0.8,
    l: 0.55,
    a: 1.,
};

struct Host {
    chosen: usize,
    tinted: bool,
    /// The picker's list is open: its backdrop takes every click, so a select is tried without.
    listed: bool,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (chosen, tinted, listed) = (self.chosen, self.tinted, self.listed);
        let ink = |colour: Hsla| tinted.then_some(colour);
        let leading = |item: MenuItem, colour: Hsla| match ink(colour) {
            Some(colour) => item.leading(Icon::Plus).tint(colour),
            None => item.leading(Icon::Plus),
        };
        let trigger = match ink(TEAL) {
            Some(colour) => Dropdown::new("picker", "Here")
                .leading(Icon::Plus)
                .tint(colour),
            None => Dropdown::new("picker", "Here").leading(Icon::Plus),
        };
        let option = |key: &'static str, title: &'static str, colour: Hsla| {
            let option = SelectOption::new(key, title).leading(Icon::Plus);
            match ink(colour) {
                Some(colour) => option.tint(colour),
                None => option,
            }
        };
        let radio = |radios: RadioChoices, key: &'static str, colour: Hsla| {
            let radios = radios.choice(key, key, "a line").leading(Icon::Plus);
            match ink(colour) {
                Some(colour) => radios.tint(colour),
                None => radios,
            }
        };
        div()
            .w(px(700.))
            .p(px(16.))
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                trigger
                    .open(listed)
                    .on_toggle(|_, _| {})
                    .item(
                        leading(MenuItem::new("pick-a", "Alpha"), TEAL)
                            .checked(chosen == 0)
                            .on_click(|_, _| {}),
                    )
                    .item(
                        leading(MenuItem::new("pick-b", "Beta"), AMBER)
                            .checked(chosen == 1)
                            .on_click(|_, _| {}),
                    )
                    .item(MenuItem::new("pick-c", "Gamma").on_click(|_, _| {})),
            )
            .child(
                Select::new(
                    "place",
                    [option("a", "Alpha", TEAL), option("b", "Beta", AMBER)],
                    chosen,
                )
                .fit(),
            )
            .child(radio(
                radio(RadioChoices::new("kind", "Kind"), "a", TEAL),
                "b",
                AMBER,
            ))
    }
}

fn host(
    tinted: bool,
    listed: bool,
    cx: &mut TestAppContext,
) -> (Entity<Host>, &mut VisualTestContext) {
    init(cx);
    let (view, cx) = cx.add_window_view(move |_, _| Host {
        chosen: 0,
        tinted,
        listed,
    });
    cx.simulate_resize(size(px(1000.), px(700.)));
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, ENTER * 2);
    (view, cx)
}

fn choose(view: &Entity<Host>, at: usize, cx: &mut VisualTestContext) {
    cx.update(|_, cx| {
        view.update(cx, |host, cx| {
            host.chosen = at;
            cx.notify();
        })
    });
    cx.update(|window, cx| window.render_frame(cx));
}

#[gpui_kit::test]
fn a_picker_button_and_its_entries_lead_with_the_icon_in_the_colour_given(cx: &mut TestAppContext) {
    let (_, cx) = host(true, true, cx);
    assert_eq!(colour("picker-leading"), Some(TEAL), "the button's icon");
    assert_eq!(colour("pick-a-leading"), Some(TEAL));
    assert_eq!(colour("pick-b-leading"), Some(AMBER));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn an_icon_with_no_colour_given_keeps_the_faint_ink_it_always_had(cx: &mut TestAppContext) {
    let (view, cx) = host(false, true, cx);
    let faint = cx.update(|_, cx| palette(cx).faint);
    for id in ["picker-leading", "pick-a-leading", "kind-a-leading"] {
        assert_eq!(colour(id), Some(faint), "{id}");
    }
    assert_eq!(
        seen("pick-c-check"),
        None,
        "an entry with no check has none"
    );
    view.update(cx, |host, cx| {
        host.listed = false;
        cx.notify();
    });
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, ENTER * 2);
    cx.update(|window, cx| window.click("place", cx));
    advance(cx, ENTER * 2);
    assert_eq!(colour("place-a-leading"), Some(faint), "an option's too");
}

#[gpui_kit::test]
fn a_select_and_radio_choices_lead_with_the_icon_in_the_colour_given(cx: &mut TestAppContext) {
    let (view, cx) = host(true, false, cx);
    assert_eq!(
        colour("place-leading"),
        Some(TEAL),
        "the chosen one's, on the field"
    );
    cx.update(|window, cx| window.click("place", cx));
    advance(cx, ENTER * 2);
    assert_eq!(colour("place-a-leading"), Some(TEAL));
    assert_eq!(colour("place-b-leading"), Some(AMBER));
    assert_eq!(colour("kind-a-leading"), Some(TEAL));
    assert_eq!(colour("kind-b-leading"), Some(AMBER));
    choose(&view, 1, cx);
    assert_eq!(
        colour("place-leading"),
        Some(AMBER),
        "the field follows the choice"
    );
    advance(cx, HOVER / 3);
    let midway = seen("place-leading").unwrap();
    assert!(midway < 1.0, "the new icon fades in over HOVER: {midway}");
    advance(cx, HOVER * 2);
    assert_eq!(seen("place-leading"), Some(1.0));
}

#[gpui_kit::test]
fn the_check_of_the_chosen_entry_moves_over_panel(cx: &mut TestAppContext) {
    let (view, cx) = host(true, true, cx);
    assert_eq!(seen("pick-a-check"), Some(1.0), "the chosen entry has it");
    assert_eq!(seen("pick-b-check"), Some(0.0));
    choose(&view, 1, cx);
    advance(cx, PANEL / 3);
    let (leaving, arriving) = (seen("pick-a-check").unwrap(), seen("pick-b-check").unwrap());
    assert!(leaving > 0.0 && leaving < 1.0, "leaving: {leaving}");
    assert!(arriving > 0.0 && arriving < 1.0, "arriving: {arriving}");
    advance(cx, PANEL * 2);
    assert_eq!(seen("pick-a-check"), Some(0.0));
    assert_eq!(seen("pick-b-check"), Some(1.0));
    assert_eq!(pending_frames(cx), 0);
}
