//! `ChoiceRow`'s field: a row that, once chosen, holds a field to the right of its label. The
//! field unfolds and fades in inside the row, the label changes and sits on the field's first
//! line, and the row's height follows over `PANEL`; un-chosen, all of it goes back.

mod common;

use std::time::Duration;

use common::{advance, init, pending_frames};
use gpui_design_system::ChoiceRow;
use gpui_design_system::motion::{HOVER, PANEL};
use gpui_design_system::seen::seen;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Bounds, Context, Entity, InteractiveElement as _, IntoElement, ParentElement, Pixels, Render,
    Styled, TestAppContext, TestSupportExt as _, VisualTestContext, Window, div, px, size,
};

struct Host {
    chosen: bool,
    toggled: usize,
    with_field: bool,
    words: &'static str,
    width: f32,
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
            // A field as a text field is drawn: padding and a border around a first line.
            let frame = div()
                .id("typed")
                .px(px(8.))
                .py(px(4.))
                .border_1()
                .child(div().id("typed-line").h(px(20.)).test_support())
                .test_support();
            row = row.field(self.words, frame).field_inset(px(5.));
        }
        div()
            .w(px(self.width))
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
    sized(chosen, with_field, reduce, ("Something else:", 400.), cx)
}

fn sized<'a>(
    chosen: bool,
    with_field: bool,
    reduce: bool,
    (words, width): (&'static str, f32),
    cx: &'a mut TestAppContext,
) -> (Entity<Host>, &'a mut VisualTestContext) {
    init(cx);
    cx.update(|cx| cx.set_reduce_motion(reduce));
    let (view, cx) = cx.add_window_view(move |_, _| Host {
        chosen,
        toggled: 0,
        with_field,
        words,
        width,
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

fn middle(id: &str, cx: &mut VisualTestContext) -> Pixels {
    let at = bounds(id, cx);
    at.top() + at.size.height / 2.
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
    assert_eq!(seen("other-field"), Some(0.0), "not shown yet");
    click("other", cx);
    advance(cx, PANEL / 3);
    let fading = seen("other-field").unwrap();
    assert!(0.0 < fading && fading < 1.0, "it fades in: {fading}");
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
    assert_eq!(seen("other-field"), Some(1.0));
    assert_eq!(typed.size.height, px(30.));
    assert_eq!(row.size.height, px(48.), "the row is as high as its field");
    assert_eq!(toggled(&view, cx), 1);
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn the_label_and_the_mark_drop_onto_the_first_line_of_the_field(cx: &mut TestAppContext) {
    let (_, cx) = host(false, true, false, cx);
    assert_eq!(label("other-label", cx), Some("Something else…".to_owned()));
    let alone = middle("other-label", cx);
    assert_eq!(
        middle("other-mark", cx),
        alone,
        "the mark is on the label's line"
    );
    click("other", cx);
    advance(cx, PANEL * 2);
    assert_eq!(label("other-label", cx), Some("Something else:".to_owned()));
    let (line, words, mark) = (
        middle("typed-line", cx),
        middle("other-label", cx),
        middle("other-mark", cx),
    );
    assert!(
        (line - words).abs() < px(1.),
        "the label is on the field's first line: {words:?} {line:?}"
    );
    assert!(
        (line - mark).abs() < px(1.),
        "so is the mark: {mark:?} {line:?}"
    );
    assert!(words - alone > px(4.), "it dropped, it did not start there");
    assert_eq!(
        label("other", cx),
        Some("Something else…".to_owned()),
        "its name stays"
    );
}

#[gpui_kit::test]
fn a_long_label_wraps_and_the_field_keeps_its_width(cx: &mut TestAppContext) {
    let long = "Something else entirely, said at some length before the field:";
    let (_, cx) = sized(true, true, false, (long, 260.), cx);
    advance(cx, PANEL);
    let (row, words, typed) = (
        bounds("other", cx),
        bounds("other-label", cx),
        bounds("typed", cx),
    );
    assert!(words.size.height >= px(40.), "the label wraps: {words:?}");
    assert!(
        typed.size.width >= px(120.),
        "the field keeps its room: {typed:?}"
    );
    assert!(
        words.right() <= typed.left(),
        "side by side: {words:?} {typed:?}"
    );
    assert!(
        typed.right() <= row.right(),
        "inside the row: {row:?} {typed:?}"
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
