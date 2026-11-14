//! `DiffLines`: the lines of a text that changed between two versions, marked `+` and `−` with
//! the palette's backgrounds, on GPUI's test platform.

mod common;

use common::{init, pending_frames};
use gpui_design_system::theme::{Palette, Shade};
use gpui_design_system::{DiffLine, DiffLines};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div, px,
};

struct Host;

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(300.)).child(DiffLines::new(
            "diff",
            [
                DiffLine::kept("Drives one browser window."),
                DiffLine::removed("Call browser_snapshot after a page changes."),
                DiffLine::added("Always call browser_snapshot after a page changes; refs are only valid for the latest one."),
            ],
        ))
    }
}

#[gpui_kit::test]
fn diff_lines_mark_added_and_removed_lines(cx: &mut TestAppContext) {
    init(cx);
    let (_, cx) = cx.add_window_view(|_, _| Host);
    cx.update(|window, cx| window.render_frame(cx));
    let said = |id: &'static str, cx: &mut gpui_kit::VisualTestContext| {
        cx.update(|window, _| window.find(id).label().map(str::to_owned))
    };
    assert_eq!(
        said("diff-line-0", cx).as_deref(),
        Some("  Drives one browser window.")
    );
    assert_eq!(
        said("diff-line-1", cx).as_deref(),
        Some("− Call browser_snapshot after a page changes.")
    );
    assert!(
        said("diff-line-2", cx)
            .is_some_and(|line| line.starts_with("+ Always call browser_snapshot")),
    );
    // A line that wraps takes the room of all its lines, and the lines stay in order.
    let (kept, added) = cx.update(|window, _| {
        (
            window.find("diff-line-0").bounds(),
            window.find("diff-line-2").bounds(),
        )
    });
    assert!(added.size.height > kept.size.height, "{added:?} / {kept:?}");
    assert!(kept.bottom() <= added.origin.y);
    assert_eq!(pending_frames(cx), 0, "a diff does not move");
}

#[test]
fn an_added_line_is_green_and_a_removed_one_red_in_both_shades() {
    for shade in [Shade::Light, Shade::Dark] {
        let p = Palette::own(shade);
        assert_eq!(DiffLine::added("x").fill(&p), Some(p.add));
        assert_eq!(DiffLine::added("x").ink(&p), p.ok);
        assert_eq!(DiffLine::removed("x").fill(&p), Some(p.del));
        assert_eq!(DiffLine::removed("x").ink(&p), p.fail);
        assert_eq!(
            DiffLine::kept("x").fill(&p),
            None,
            "a kept line has no fill"
        );
        assert_eq!(DiffLine::added("x").sign(), "+");
        assert_eq!(DiffLine::removed("x").sign(), "−");
        assert_eq!(DiffLine::kept("x").sign(), " ");
    }
}
