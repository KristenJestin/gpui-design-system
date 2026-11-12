//! `Tag`: the small bordered word after a title, drawn once for every place that has one, on
//! GPUI's test platform.

mod common;

use common::{advance, init, pending_frames};
use gpui_design_system::motion::{HOVER, TOOLTIP};
use gpui_design_system::{ListRow, Section, Tag};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, IntoElement, ParentElement, Render, SharedString, Styled, TestAppContext, Window, div,
    px, size,
};

const PATH: &str =
    "/home/someone/documents/a-folder-with-a-long-name/another/and-another/deep/file.md";

struct Tagged {
    text: SharedString,
}

impl Render for Tagged {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(300.))
            .flex()
            .child(Tag::new("changed", self.text.clone()))
    }
}

#[gpui_kit::test]
fn a_tag_is_its_own_element_labelled_with_its_text(cx: &mut TestAppContext) {
    init(cx);
    let (_, cx) = cx.add_window_view(|_, _| Tagged {
        text: "Changed".into(),
    });
    cx.update(|window, cx| window.render_frame(cx));
    assert_eq!(
        cx.update(|window, _| window.find("changed").label().map(str::to_owned)),
        Some("Changed".to_owned())
    );
}

#[gpui_kit::test]
fn a_tag_that_changes_fades_in_and_then_stops_asking_for_frames(cx: &mut TestAppContext) {
    init(cx);
    let (view, cx) = cx.add_window_view(|_, _| Tagged { text: "New".into() });
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, HOVER);
    assert_eq!(pending_frames(cx), 0, "a first text is simply there");

    cx.update(|_, cx| {
        view.update(cx, |tagged, cx| {
            tagged.text = "Changed".into();
            cx.notify()
        })
    });
    cx.update(|window, cx| window.render_frame(cx));
    assert!(pending_frames(cx) > 0, "the new text fades in");
    advance(cx, HOVER * 2);
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn with_reduced_motion_a_tag_jumps(cx: &mut TestAppContext) {
    init(cx);
    cx.update(|cx| cx.set_reduce_motion(true));
    let (view, cx) = cx.add_window_view(|_, _| Tagged { text: "New".into() });
    cx.update(|window, cx| window.render_frame(cx));
    cx.update(|_, cx| {
        view.update(cx, |tagged, cx| {
            tagged.text = "Changed".into();
            cx.notify()
        })
    });
    cx.update(|window, cx| window.render_frame(cx));
    assert_eq!(pending_frames(cx), 0, "nothing fades");
}

#[gpui_kit::test]
fn a_long_tag_is_cut_at_160_px_and_says_itself_whole(cx: &mut TestAppContext) {
    init(cx);
    let (_, cx) = cx.add_window_view(|_, _| Tagged { text: PATH.into() });
    cx.simulate_resize(size(px(800.), px(400.)));
    cx.update(|window, cx| window.render_frame(cx));
    let width = cx.update(|window, _| window.find("changed").bounds().size.width);
    assert!(width <= px(160.), "{width:?}");
    cx.update(|window, cx| window.hover("changed", cx));
    advance(cx, TOOLTIP + HOVER * 2);
    cx.update(|window, cx| window.render_frame(cx));
    assert_eq!(
        cx.update(|window, _| window
            .find("changed-tooltip".to_owned())
            .label()
            .map(str::to_owned)),
        Some(PATH.to_owned()),
        "the whole text is its tooltip"
    );
}

struct Beside;

impl Render for Beside {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(300.))
            .flex()
            .flex_col()
            .child(ListRow::new("row", "Skill").tag("On"))
            .child(Section::new("section", "Skills").tag("Default"))
    }
}

#[gpui_kit::test]
fn a_list_row_and_a_section_keep_the_ids_of_their_tags(cx: &mut TestAppContext) {
    init(cx);
    let (_, cx) = cx.add_window_view(|_, _| Beside);
    cx.update(|window, cx| window.render_frame(cx));
    for (id, text) in [("row-tag-0", "On"), ("section-tag", "Default")] {
        assert_eq!(
            cx.update(|window, _| window.find(id.to_owned()).label().map(str::to_owned)),
            Some(text.to_owned()),
            "{id}"
        );
    }
}
