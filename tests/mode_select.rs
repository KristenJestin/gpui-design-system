//! A mode in a select (`Select`), one picker with three entries each with its one
//! line, on GPUI's test platform: it says the entry chosen and reports the one picked.

mod common;

use std::cell::Cell;
use std::rc::Rc;

use common::{advance, init, pending_frames};
use gpui_design_system::motion::{ENTER, PANEL};
use gpui_design_system::{Select, SelectOption};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div, px, size,
};

struct Host {
    chosen: Rc<Cell<usize>>,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (chosen, this) = (self.chosen.clone(), cx.entity().downgrade());
        div().p(px(16.)).w(px(240.)).child(
            Select::new(
                "mode",
                [
                    SelectOption::new("ask", "Ask").line("Every tool call waits for you"),
                    SelectOption::new("auto", "Auto").line("Safe calls go, the rest waits"),
                    SelectOption::new("full", "Full").line("Every call goes"),
                ],
                self.chosen.get(),
            )
            .on_choose(move |index, _, cx| {
                chosen.set(index);
                this.update(cx, |_, cx| cx.notify()).ok();
            }),
        )
    }
}

#[gpui_kit::test]
fn the_mode_select_says_its_entry_and_picks_another(cx: &mut TestAppContext) {
    init(cx);
    let chosen = Rc::new(Cell::new(0));
    let given = chosen.clone();
    let (_, cx) = cx.add_window_view(move |_, _| Host { chosen: given });
    cx.simulate_resize(size(px(900.), px(700.)));
    cx.update(|window, cx| window.render_frame(cx));
    cx.update(|window, _| assert_eq!(window.find("mode").label(), Some("Ask")));
    cx.update(|window, cx| window.click("mode", cx));
    advance(cx, ENTER * 2);
    // Each entry says its meaning in one line.
    cx.update(|window, _| {
        for (key, line) in [
            ("ask", "Every tool call waits for you"),
            ("auto", "Safe calls go, the rest waits"),
            ("full", "Every call goes"),
        ] {
            assert_eq!(
                window.find(format!("mode-{key}-line")).label(),
                Some(line),
                "{key}"
            );
        }
        assert_eq!(window.find("mode-ask").selected(), Some(true));
    });
    cx.update(|window, cx| window.click("mode-auto", cx));
    assert_eq!(chosen.get(), 1);
    advance(cx, PANEL * 2);
    cx.update(|window, _| assert_eq!(window.find("mode").label(), Some("Auto")));
    assert_eq!(pending_frames(cx), 0);
}
