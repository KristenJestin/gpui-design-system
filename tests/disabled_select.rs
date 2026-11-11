//! A `Select` (and the `ModelChoice` made of two) that turns disabled fades over `HOVER` instead
//! of jumping, takes no click and no list while it is, and stays put at rest. The set-here mark
//! beside it keeps working.

mod common;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use common::{advance, init, pending_frames};
use gpui_design_system::motion::{ENTER, HOVER};
use gpui_design_system::{Chosen, ModelChoice, ModelEntry, Select, SelectOption, SetHere};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, Entity, IntoElement, ParentElement, Render, Styled, TestAppContext, VisualTestContext,
    Window, div, px,
};

/// A select and a model choice, both disabled when `off`, that say what was chosen and reset.
struct Host {
    off: bool,
    chosen: Rc<Cell<usize>>,
    models: Rc<RefCell<Vec<Chosen>>>,
    reset: Rc<Cell<usize>>,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (chosen, models, reset) =
            (self.chosen.clone(), self.models.clone(), self.reset.clone());
        let select = Select::new(
            "tracker",
            [
                SelectOption::new("github", "GitHub"),
                SelectOption::new("jira", "Jira"),
            ],
            0,
        )
        .disabled(self.off)
        .on_choose(move |index, _, _| chosen.set(index));
        let model = ModelChoice::new("judge", "Model", Chosen::First)
            .inherited("Opus for Claude")
            .models([ModelEntry::new(
                "claude",
                "Claude",
                Some("opus".into()),
                "Opus",
            )])
            .compact()
            .disabled(self.off)
            .on_choose(move |chosen, _, _, _| models.borrow_mut().push(chosen));
        div()
            .w(px(700.))
            .p(px(16.))
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                SetHere::new("mark", select, "set here", "Use Settings' value")
                    .set(true)
                    .on_reset(move |_, _| reset.set(reset.get() + 1)),
            )
            .child(model)
    }
}

fn host(off: bool, cx: &mut TestAppContext) -> (Entity<Host>, &mut VisualTestContext) {
    init(cx);
    let (view, cx) = cx.add_window_view(move |_, _| Host {
        off,
        chosen: Rc::new(Cell::new(usize::MAX)),
        models: Rc::new(RefCell::new(Vec::new())),
        reset: Rc::new(Cell::new(0)),
    });
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

fn turn(view: &Entity<Host>, off: bool, cx: &mut VisualTestContext) {
    cx.update(|_, cx| {
        view.update(cx, |this, cx| {
            this.off = off;
            cx.notify();
        })
    });
    cx.update(|window, cx| window.render_frame(cx));
}

/// How far the select `id` is enabled (0 to 1).
fn enabled(id: &str, _: &mut VisualTestContext) -> f32 {
    gpui_design_system::seen::seen(format!("{id}-root")).expect("a select was drawn")
}

fn listed(id: &str, cx: &mut VisualTestContext) -> bool {
    let id = id.to_owned();
    cx.update(|window, cx| {
        window.render_frame(cx);
        window.try_find(id).is_some_and(|found| found.visible())
    })
}

#[gpui_kit::test]
fn a_select_turned_disabled_fades_and_takes_no_click(cx: &mut TestAppContext) {
    let (view, cx) = host(false, cx);
    assert_eq!(enabled("tracker", cx), 1.0);
    turn(&view, true, cx);
    advance(cx, HOVER / 3);
    let part = enabled("tracker", cx);
    assert!(0.0 < part && part < 1.0, "part way, not a jump: {part}");
    advance(cx, HOVER * 2);
    assert_eq!(enabled("tracker", cx), 0.0);
    assert_eq!(pending_frames(cx), 0, "and it stops asking for frames");

    cx.update(|window, cx| window.click("tracker", cx));
    advance(cx, ENTER * 2);
    assert!(!listed("tracker-menu", cx), "no list opens");
    assert_eq!(
        view.read_with(cx, |this, _| this.chosen.get()),
        usize::MAX,
        "nothing was chosen"
    );

    turn(&view, false, cx);
    advance(cx, HOVER / 3);
    let back = enabled("tracker", cx);
    assert!(0.0 < back && back < 1.0, "and fades back: {back}");
    advance(cx, HOVER * 2);
    cx.update(|window, cx| window.click("tracker", cx));
    advance(cx, ENTER * 2);
    assert!(listed("tracker-menu", cx), "enabled again, it opens");
}

#[gpui_kit::test]
fn a_select_that_is_open_when_it_turns_disabled_closes(cx: &mut TestAppContext) {
    let (view, cx) = host(false, cx);
    cx.update(|window, cx| window.click("tracker", cx));
    advance(cx, ENTER * 2);
    assert!(listed("tracker-menu", cx));
    turn(&view, true, cx);
    advance(cx, ENTER * 2);
    assert!(!listed("tracker-menu", cx));
    turn(&view, false, cx);
    advance(cx, ENTER * 2);
    assert!(!listed("tracker-menu", cx), "it does not come back open");
}

#[gpui_kit::test]
fn a_disabled_select_stays_put_at_rest(cx: &mut TestAppContext) {
    let (_, cx) = host(true, cx);
    assert_eq!(enabled("tracker", cx), 0.0, "first drawn disabled: faded");
    assert_eq!(pending_frames(cx), 0, "no frame asked at rest");
    advance(cx, HOVER * 3);
    assert_eq!(enabled("tracker", cx), 0.0);
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_disabled_model_choice_fades_its_select_and_takes_no_click(cx: &mut TestAppContext) {
    let (view, cx) = host(false, cx);
    turn(&view, true, cx);
    advance(cx, HOVER / 3);
    let part = enabled("judge-model", cx);
    assert!(0.0 < part && part < 1.0, "part way: {part}");
    advance(cx, HOVER * 2);
    assert_eq!(enabled("judge-model", cx), 0.0);
    cx.update(|window, cx| window.click("judge-model", cx));
    advance(cx, ENTER * 2);
    assert!(!listed("judge-model-menu", cx));
    assert!(view.read_with(cx, |this, _| this.models.borrow().is_empty()));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn the_set_here_mark_beside_a_disabled_select_stays_usable(cx: &mut TestAppContext) {
    let (view, cx) = host(true, cx);
    cx.update(|window, cx| window.click("mark-reset", cx));
    assert_eq!(view.read_with(cx, |this, _| this.reset.get()), 1);
}

#[gpui_kit::test]
fn a_disabled_select_takes_no_focus(cx: &mut TestAppContext) {
    let (view, cx) = host(true, cx);
    cx.update(|window, cx| window.click("tracker", cx));
    cx.update(|window, cx| assert!(window.focused(cx).is_none(), "a click does not focus it"));

    turn(&view, false, cx);
    advance(cx, HOVER * 2);
    cx.update(|window, cx| window.click("tracker", cx));
    cx.update(|window, cx| {
        assert!(
            window.focused(cx).is_some(),
            "enabled, it opens and has focus"
        )
    });

    turn(&view, true, cx);
    cx.update(|window, cx| {
        assert!(
            window.focused(cx).is_none(),
            "turning disabled takes its focus away"
        )
    });
}
