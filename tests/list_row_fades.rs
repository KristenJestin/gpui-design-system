//! `ListRow`: a title that changes fades from the old to the new as a tree row's does, a tag that
//! comes beside it unfolds with its gap and one that leaves folds away, reduced motion jumps, and
//! a row at rest draws nothing and asks for no frame. On GPUI's test platform.

mod common;

use common::{advance, init, pending_frames};
use gpui_design_system::ListRow;
use gpui_design_system::motion::{HOVER, PANEL};
use gpui_design_system::seen::seen;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Bounds, Context, Entity, IntoElement, ParentElement, Pixels, Render, SharedString, Styled,
    TestAppContext, VisualTestContext, Window, div, px,
};

/// A row of a list, whose title and tags the view changes.
struct Listed {
    title: SharedString,
    tags: Vec<SharedString>,
}

impl Render for Listed {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let row = self
            .tags
            .iter()
            .fold(ListRow::new("row", self.title.clone()), |row, tag| {
                row.tag(tag.clone())
            });
        div().w(px(360.)).child(row)
    }
}

fn listed<'a>(
    title: &'static str,
    tags: &[&'static str],
    reduce: bool,
    cx: &'a mut TestAppContext,
) -> (Entity<Listed>, &'a mut VisualTestContext) {
    init(cx);
    let tags = tags.iter().map(|tag| (*tag).into()).collect();
    let (view, cx) = cx.add_window_view(|_, _| Listed {
        title: title.into(),
        tags,
    });
    cx.update(|_, cx| cx.set_reduce_motion(reduce));
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, HOVER);
    (view, cx)
}

fn change(
    view: &Entity<Listed>,
    cx: &mut VisualTestContext,
    to: impl FnOnce(&mut Listed) + 'static,
) {
    cx.update(|_, cx| {
        view.update(cx, |listed, cx| {
            to(listed);
            cx.notify()
        })
    });
    cx.update(|window, cx| window.render_frame(cx));
}

/// The place of the tag `row-tag-<index>`: its own while it rests, the room it unfolds or folds in
/// (`-slot`, which clips the tag) while it moves. `None` while it is not drawn at all.
fn tag_place(index: usize, cx: &mut VisualTestContext) -> Option<Bounds<Pixels>> {
    cx.update(|window, _| {
        let id = format!("row-tag-{index}");
        window
            .try_find(format!("{id}-slot"))
            .or_else(|| window.try_find(id))
            .map(|place| place.bounds())
    })
}

fn tag_width(index: usize, cx: &mut VisualTestContext) -> Option<Pixels> {
    tag_place(index, cx).map(|place| place.size.width)
}

/// How far the first tag's place starts after the title.
fn gap_before_tag(cx: &mut VisualTestContext) -> Pixels {
    let place = tag_place(0, cx).expect("a tag");
    cx.update(|window, _| place.left() - window.find("row-title").bounds().right())
}

#[gpui_kit::test]
fn a_changed_list_row_title_is_seen_partway_between_0_and_1(cx: &mut TestAppContext) {
    let (view, cx) = listed("untitled", &[], false, cx);
    assert_eq!(
        seen("row-title"),
        Some(1.0),
        "a first title is simply there"
    );

    change(&view, cx, |listed| listed.title = "Release notes".into());
    advance(cx, HOVER / 2);
    let partway = seen("row-title").expect("a title that changes is noted");
    assert!(
        0.0 < partway && partway < 1.0,
        "strictly between 0 and 1 partway: {partway}"
    );

    advance(cx, HOVER * 2);
    assert_eq!(seen("row-title"), Some(1.0), "at rest at 1");
}

#[gpui_kit::test]
fn with_reduced_motion_a_changed_list_row_title_jumps(cx: &mut TestAppContext) {
    let (view, cx) = listed("untitled", &[], true, cx);

    change(&view, cx, |listed| listed.title = "Release notes".into());
    assert_eq!(seen("row-title"), Some(1.0), "never seen partway");
    advance(cx, HOVER / 2);
    assert_eq!(seen("row-title"), Some(1.0));
    assert_eq!(pending_frames(cx), 0, "and it asks for no frame");
}

#[gpui_kit::test]
fn a_tag_that_comes_beside_the_title_unfolds_with_its_gap(cx: &mut TestAppContext) {
    let (view, cx) = listed("Release notes", &[], false, cx);
    let title = cx.update(|window, _| window.find("row-title").bounds());
    assert_eq!(tag_width(0, cx), None, "no tag, nothing drawn");

    change(&view, cx, |listed| listed.tags = vec!["draft".into()]);
    advance(cx, PANEL / 2);
    let partway = tag_width(0, cx).expect("the tag is drawn as it comes");
    let gap_partway = gap_before_tag(cx);
    advance(cx, PANEL * 2);
    let whole = tag_width(0, cx).expect("and stays");
    assert!(
        px(0.) < partway && partway < whole,
        "partway between nothing and whole: {partway:?} of {whole:?}"
    );
    let at_rest = cx.update(|window, _| window.find("row-title").bounds());
    assert_eq!(
        at_rest, title,
        "the title is where it was, the tag came after it"
    );
    assert!(
        px(0.) < gap_partway && gap_partway < px(6.),
        "the gap unfolds with it: {gap_partway:?}"
    );
    assert_eq!(
        gap_before_tag(cx),
        px(6.),
        "at rest it sits a gap after the title"
    );
}

#[gpui_kit::test]
fn a_tag_that_leaves_folds_away_with_its_gap(cx: &mut TestAppContext) {
    let (view, cx) = listed("Release notes", &["draft"], false, cx);
    let whole = tag_width(0, cx).expect("a first tag is simply there");

    change(&view, cx, |listed| listed.tags.clear());
    advance(cx, PANEL / 2);
    let partway = tag_width(0, cx).expect("it is still drawn as it leaves");
    assert!(
        px(0.) < partway && partway < whole,
        "partway between whole and nothing: {partway:?} of {whole:?}"
    );
    assert!(gap_before_tag(cx) < px(6.), "and its gap folds with it");
    assert!(gap_before_tag(cx) < px(6.), "and its gap folds with it");

    advance(cx, PANEL * 2);
    assert_eq!(tag_width(0, cx), None, "and gone");
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn with_reduced_motion_a_tag_that_comes_or_leaves_jumps(cx: &mut TestAppContext) {
    let (view, cx) = listed("Release notes", &[], true, cx);

    change(&view, cx, |listed| listed.tags = vec!["draft".into()]);
    let whole = tag_width(0, cx).expect("it is there at once");
    assert!(whole > px(0.));
    assert_eq!(pending_frames(cx), 0, "no frame asked");

    change(&view, cx, |listed| listed.tags.clear());
    assert_eq!(tag_width(0, cx), None, "and gone at once");
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_list_row_at_rest_draws_no_frame(cx: &mut TestAppContext) {
    let (view, cx) = listed("Release notes", &["draft"], false, cx);
    advance(cx, PANEL * 2);
    assert_eq!(pending_frames(cx), 0, "a row with a title and a tag");

    change(&view, cx, |listed| listed.title = "Changelog".into());
    advance(cx, PANEL * 2);
    assert_eq!(pending_frames(cx), 0, "and once its title has faded");

    change(&view, cx, |listed| {
        listed.tags = vec!["draft".into(), "text".into()]
    });
    advance(cx, PANEL * 2);
    assert_eq!(pending_frames(cx), 0, "and once a tag has unfolded");
}
