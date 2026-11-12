//! `ChoiceRow`'s field: a row that, once chosen, holds a field to the right of its label. The
//! field unfolds and fades in inside the row, the label changes and sits on the field's first
//! line, and the row's height follows over `PANEL`; un-chosen, all of it goes back.

mod common;

use std::time::Duration;

use common::{advance, init, pending_frames};
use gpui_design_system::ChoiceRow;
use gpui_design_system::motion::{HOVER, PANEL};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Bounds, Context, Entity, InteractiveElement as _, IntoElement, ParentElement, Pixels, Render,
    Styled, TestAppContext, TestSupportExt as _, VisualTestContext, Window, div, px, size,
};

struct Host {
    chosen: bool,
    toggled: usize,
    with_field: bool,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        let mut row = ChoiceRow::new("other", "Something else…")
            .faint()
            .chosen(self.chosen)
            .on_click(move |_, cx| {
                this.update(cx, |host, cx| {
                    host.chosen = !host.chosen;
                    host.toggled += 1;
                    cx.notify();
                })
                .ok();
            });
        if self.with_field {
            row = row.field(
                "Something else:",
                div().id("typed").h(px(30.)).test_support(),
            );
        }
        div()
            .w(px(400.))
            .flex()
            .flex_col()
            .child(row)
            .child(div().id("after").h(px(20.)).test_support())
    }
}

fn host(
    chosen: bool,
    with_field: bool,
    reduce: bool,
    cx: &mut TestAppContext,
) -> (Entity<Host>, &mut VisualTestContext) {
    init(cx);
    cx.update(|cx| cx.set_reduce_motion(reduce));
    let (view, cx) = cx.add_window_view(move |_, _| Host {
        chosen,
        toggled: 0,
        with_field,
    });
    cx.simulate_resize(size(px(600.), px(400.)));
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

fn bounds(id: &str, cx: &mut VisualTestContext) -> Bounds<Pixels> {
    let id = id.to_owned();
    cx.update(|window, _| window.find(id).bounds())
}

fn present(id: &str, cx: &mut VisualTestContext) -> bool {
    let id = id.to_owned();
    cx.update(|window, _| window.try_find(id).is_some())
}

fn label(id: &str, cx: &mut VisualTestContext) -> Option<String> {
    let id = id.to_owned();
    cx.update(|window, _| window.find(id).label().map(str::to_owned))
}

fn click(id: &str, cx: &mut VisualTestContext) {
    let id = id.to_owned();
    cx.update(|window, cx| window.click(id, cx));
}

fn toggled(view: &Entity<Host>, cx: &mut VisualTestContext) -> usize {
    cx.update(|_, cx| view.read(cx).toggled)
}

#[gpui_kit::test]
fn the_field_is_not_there_while_the_row_is_not_chosen(cx: &mut TestAppContext) {
    let (_, cx) = host(false, true, false, cx);
    assert!(!present("typed", cx));
    assert_eq!(bounds("other", cx).size.height, px(38.), "a row as any");
    advance(cx, PANEL);
    assert_eq!(pending_frames(cx), 0, "nothing runs at rest");
}

#[gpui_kit::test]
fn the_field_unfolds_and_fades_in_to_the_right_of_the_label_once_chosen(cx: &mut TestAppContext) {
    let (view, cx) = host(false, true, false, cx);
    let (before, after) = (
        bounds("other", cx).size.height,
        bounds("after", cx).origin.y,
    );
    click("other", cx);
    advance(cx, PANEL / 3);
    let height = bounds("other", cx).size.height;
    assert!(
        before < height,
        "the row grows part way: {before:?} {height:?}"
    );
    assert!(
        bounds("after", cx).origin.y > after,
        "what is under follows"
    );
    advance(cx, PANEL * 2);
    let (row, words, typed) = (
        bounds("other", cx),
        bounds("other-label", cx),
        bounds("typed", cx),
    );
    assert!(words.right() <= typed.left(), "right of the words");
    assert!(
        typed.right() <= row.right() && row.right() - typed.right() <= px(12.),
        "it takes the rest of the row: {row:?} {typed:?}"
    );
    assert_eq!(typed.size.height, px(30.));
    assert_eq!(row.size.height, px(48.), "the row is as high as its field");
    assert_eq!(toggled(&view, cx), 1);
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn the_label_changes_and_sits_on_the_first_line_of_the_field(cx: &mut TestAppContext) {
    let (_, cx) = host(false, true, false, cx);
    assert_eq!(label("other-label", cx), Some("Something else…".to_owned()));
    click("other", cx);
    advance(cx, PANEL * 2);
    assert_eq!(label("other-label", cx), Some("Something else:".to_owned()));
    let (words, typed) = (bounds("other-label", cx), bounds("typed", cx));
    assert!(
        typed.top() < words.top() + px(8.) && words.top() < typed.top() + px(8.),
        "on its first line: {words:?} {typed:?}"
    );
    assert_eq!(
        label("other", cx),
        Some("Something else…".to_owned()),
        "its name stays"
    );
}

#[gpui_kit::test]
fn the_field_folds_away_and_the_label_comes_back_once_unchosen(cx: &mut TestAppContext) {
    let (_, cx) = host(true, true, false, cx);
    assert!(
        present("typed", cx),
        "chosen at its first frame: just there"
    );
    assert_eq!(bounds("other", cx).size.height, px(48.));
    click("other-label", cx);
    advance(cx, PANEL / 3);
    let part = bounds("other", cx).size.height;
    assert!(px(38.) < part && part < px(48.), "folds part way: {part:?}");
    advance(cx, PANEL * 2);
    assert!(!present("typed", cx));
    assert_eq!(bounds("other", cx).size.height, px(38.));
    assert_eq!(label("other-label", cx), Some("Something else…".to_owned()));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_click_inside_the_field_does_not_toggle_the_row(cx: &mut TestAppContext) {
    let (view, cx) = host(true, true, false, cx);
    click("typed", cx);
    advance(cx, HOVER);
    assert_eq!(toggled(&view, cx), 0, "the row did not hear it");
    assert!(present("typed", cx));
    click("other-label", cx);
    assert_eq!(toggled(&view, cx), 1, "the rest of the row still does");
}

#[gpui_kit::test]
fn a_row_without_a_field_is_unchanged(cx: &mut TestAppContext) {
    let (_, cx) = host(false, false, false, cx);
    click("other", cx);
    advance(cx, PANEL * 2);
    assert!(!present("typed", cx));
    assert_eq!(bounds("other", cx).size.height, px(38.));
    assert!(
        bounds("other-label", cx).size.width > px(0.),
        "its label is there"
    );
}

#[gpui_kit::test]
fn the_field_jumps_with_reduced_motion(cx: &mut TestAppContext) {
    let (_, cx) = host(false, true, true, cx);
    click("other", cx);
    advance(cx, Duration::from_millis(16));
    assert!(present("typed", cx));
    assert_eq!(bounds("other", cx).size.height, px(48.));
    click("other-label", cx);
    advance(cx, Duration::from_millis(16));
    assert!(!present("typed", cx));
    assert_eq!(bounds("other", cx).size.height, px(38.));
    assert_eq!(pending_frames(cx), 0);
}
