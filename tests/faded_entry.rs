//! A `Subpage` entry that is faded: a row kept in its place whose line is a link the application
//! handles (a feature that is off elsewhere), the fade and the change of line moving over `PANEL`.

mod common;

use common::{advance, init, pending_frames};
use gpui_design_system::motion::{HOVER, PANEL};
use gpui_design_system::{Icon, Subpage};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled, TestAppContext,
    VisualTestContext, Window, div, point, px,
};

/// A list of three entries, the keys in `off` faded, that says what it was told.
struct Features {
    off: Vec<&'static str>,
    opened: Vec<Option<SharedString>>,
    linked: Vec<SharedString>,
}

impl Render for Features {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (opened, linked) = (cx.entity().downgrade(), cx.entity().downgrade());
        let mut subpage = Subpage::new("feat", "Features")
            .entry("review", Icon::Check, "Review", "On", "Reads the changes")
            .entry("specs", Icon::Text, "Specs", "On", "Writes the spec first")
            .entry("notes", Icon::Plus, "Notes", "On", "Keeps notes")
            .on_open(move |key, _, cx| {
                opened
                    .update(cx, |this, _| this.opened.push(key.clone()))
                    .ok();
            })
            .on_link(move |key, _, cx| {
                linked
                    .update(cx, |this, _| this.linked.push(key.clone()))
                    .ok();
            });
        for key in &self.off {
            subpage = subpage.faded(*key, "Off in Settings");
        }
        div().w(px(600.)).child(subpage)
    }
}

fn features<'a>(
    off: &[&'static str],
    cx: &'a mut TestAppContext,
) -> (Entity<Features>, &'a mut VisualTestContext) {
    init(cx);
    let off = off.to_vec();
    let (view, cx) = cx.add_window_view(move |_, _| Features {
        off,
        opened: Vec::new(),
        linked: Vec::new(),
    });
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

fn turn(view: &Entity<Features>, off: &[&'static str], cx: &mut VisualTestContext) {
    let off = off.to_vec();
    cx.update(|_, cx| {
        view.update(cx, |this, cx| {
            this.off = off;
            cx.notify();
        })
    });
    cx.update(|window, cx| window.render_frame(cx));
}

fn top(id: &str, cx: &mut VisualTestContext) -> gpui_kit::Pixels {
    let id = id.to_owned();
    cx.update(|window, _| window.find(id).bounds().origin.y)
}

/// How far the link of an entry says it is faded (0 to 1), `None` while it is not drawn.
fn faded(key: &str, cx: &mut VisualTestContext) -> Option<f32> {
    let id = format!("feat-{key}-off");
    cx.update(|window, _| {
        window
            .try_find(id)
            .and_then(|found| found.value().and_then(|v| v.parse().ok()))
    })
}

#[gpui_kit::test]
fn a_faded_entry_keeps_its_place_in_the_list(cx: &mut TestAppContext) {
    let (_, cx) = features(&["specs"], cx);
    let (review, specs, notes) = (
        top("feat-review", cx),
        top("feat-specs", cx),
        top("feat-notes", cx),
    );
    assert!(
        review < specs && specs < notes,
        "declared order: {review:?} {specs:?} {notes:?}"
    );
    // The same place as when nothing is faded: no row moved.
    let (_, cx) = features(&[], cx);
    assert_eq!(top("feat-specs", cx), specs);
    assert_eq!(top("feat-notes", cx), notes);
}

#[gpui_kit::test]
fn a_faded_entry_says_a_link_in_place_of_its_line_and_the_link_is_clickable(
    cx: &mut TestAppContext,
) {
    let (view, cx) = features(&["specs"], cx);
    cx.update(|window, _| {
        assert_eq!(
            window.find("feat-specs-off").label(),
            Some("Off in Settings")
        );
        assert!(
            window.try_find("feat-review-off").is_none(),
            "only the faded one"
        );
    });
    assert_eq!(faded("specs", cx), Some(1.0));

    cx.update(|window, cx| window.click("feat-specs-off", cx));
    assert_eq!(
        view.read_with(cx, |this, _| this.linked.clone()),
        vec![SharedString::from("specs")]
    );
    assert!(
        view.read_with(cx, |this, _| this.opened.is_empty()),
        "the link opens nothing of the list"
    );
}

#[gpui_kit::test]
fn a_faded_entry_opens_no_page(cx: &mut TestAppContext) {
    let (view, cx) = features(&["specs"], cx);
    cx.update(|window, cx| window.click("feat-specs", cx));
    assert!(
        view.read_with(cx, |this, _| this.opened.is_empty()),
        "a click on the row is not an opening"
    );
    cx.update(|window, cx| window.click("feat-review", cx));
    assert_eq!(
        view.read_with(cx, |this, _| this.opened.clone()),
        vec![Some(SharedString::from("review"))]
    );
}

#[gpui_kit::test]
fn a_faded_entry_says_its_state_once_in_its_link_and_not_as_a_tag(cx: &mut TestAppContext) {
    let (_, cx) = features(&["specs"], cx);
    cx.update(|window, _| {
        assert!(window.try_find("feat-specs-tag-0").is_none());
        assert!(window.try_find("feat-review-tag-0").is_some());
    });
}

#[gpui_kit::test]
fn an_entry_that_turns_off_fades_and_its_line_gives_way_to_the_link_over_a_panel(
    cx: &mut TestAppContext,
) {
    let (view, cx) = features(&[], cx);
    assert_eq!(faded("specs", cx), None, "no link while it is on");
    cx.update(|window, _| assert!(window.try_find("feat-specs-line").is_some()));

    turn(&view, &["specs"], cx);
    advance(cx, PANEL / 2);
    let part = faded("specs", cx).expect("the link is drawn while it comes");
    assert!(0.0 < part && part < 1.0, "part way: {part}");
    cx.update(|window, _| {
        assert!(
            window.try_find("feat-specs-line").is_some(),
            "the line is still there while it gives way"
        );
    });
    advance(cx, PANEL * 2);
    assert_eq!(faded("specs", cx), Some(1.0));
    assert_eq!(pending_frames(cx), 0, "and it stops asking for frames");
}

#[gpui_kit::test]
fn an_entry_that_turns_back_on_fades_back_and_its_link_is_gone_once_it_has(
    cx: &mut TestAppContext,
) {
    let (view, cx) = features(&["specs"], cx);
    turn(&view, &[], cx);
    advance(cx, PANEL / 2);
    let part = faded("specs", cx).expect("the link is drawn while it leaves");
    assert!(0.0 < part && part < 1.0, "part way: {part}");
    cx.update(|window, _| {
        assert_eq!(
            window.find("feat-specs-off").label(),
            Some("Off in Settings"),
            "with its last words"
        );
    });
    advance(cx, PANEL * 2);
    assert_eq!(faded("specs", cx), None);
    cx.update(|window, _| {
        assert!(
            window.try_find("feat-specs-tag-0").is_some(),
            "its tag is back"
        );
    });
    assert_eq!(pending_frames(cx), 0);
    // It opens again.
    cx.update(|window, cx| window.click("feat-specs", cx));
    assert_eq!(
        view.read_with(cx, |this, _| this.opened.clone()),
        vec![Some(SharedString::from("specs"))]
    );
}

#[gpui_kit::test]
fn with_reduced_motion_an_entry_turns_off_at_once(cx: &mut TestAppContext) {
    let (view, cx) = features(&[], cx);
    cx.update(|_, cx| cx.set_reduce_motion(true));
    turn(&view, &["specs"], cx);
    assert_eq!(faded("specs", cx), Some(1.0));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_list_with_faded_entries_draws_nothing_at_rest(cx: &mut TestAppContext) {
    let (_, cx) = features(&["specs", "notes"], cx);
    assert_eq!(pending_frames(cx), 0);
    advance(cx, PANEL * 2);
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn the_link_of_a_faded_entry_fades_its_ink_in_and_out_on_hover(cx: &mut TestAppContext) {
    let (_, cx) = features(&["specs"], cx);
    assert_eq!(
        pending_frames(cx),
        0,
        "nothing moves before the pointer comes"
    );

    cx.update(|window, cx| window.hover("feat-specs-off", cx));
    assert!(
        pending_frames(cx) > 0,
        "the ink is on its way, not switched at once"
    );
    advance(cx, HOVER * 2);
    assert_eq!(pending_frames(cx), 0, "and it stops once it has arrived");

    cx.simulate_mouse_move(point(px(600.), px(400.)), None, Default::default());
    assert!(pending_frames(cx) > 0, "it goes out the same way");
    advance(cx, HOVER * 2);
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn with_reduced_motion_the_link_of_a_faded_entry_changes_ink_at_once(cx: &mut TestAppContext) {
    let (_, cx) = features(&["specs"], cx);
    cx.update(|_, cx| cx.set_reduce_motion(true));
    cx.update(|window, cx| window.hover("feat-specs-off", cx));
    advance(cx, std::time::Duration::from_millis(16));
    assert_eq!(pending_frames(cx), 0);
}
