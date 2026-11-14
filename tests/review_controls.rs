//! The controls a list to review is made of, on GPUI's test platform: a search field with its
//! clear button, filter chips with their counts, a checkbox with three states and a compact
//! select whose pill follows its value.

mod common;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use common::{advance, init, pending_frames};
use gpui_design_system::motion::{ENTER, HOVER, PANEL};
use gpui_design_system::seen::{colour, seen};
use gpui_design_system::{
    CheckState, Checkbox, FilterChip, FilterChips, SearchField, Select, SelectOption, Tone,
    follow_theme, palette,
};
use gpui_kit::base::input::InputState;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AppContext as _, Context, Entity, IntoElement, ParentElement, Render, Styled, TestAppContext,
    VisualTestContext, Window, div, px,
};

struct Search {
    input: Entity<InputState>,
}

impl Render for Search {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(400.)).p(px(12.)).child(
            SearchField::new("find", &self.input, "Search items")
                .placeholder("Search 35 items by name"),
        )
    }
}

fn search(cx: &mut TestAppContext) -> (Entity<Search>, &mut VisualTestContext) {
    init(cx);
    let (view, cx) = cx.add_window_view(|window, cx| Search {
        input: cx.new(|cx| {
            let mut input = InputState::new(window, cx);
            follow_theme(&mut input, cx);
            input
        }),
    });
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

fn type_in(view: &Entity<Search>, text: &str, cx: &mut VisualTestContext) {
    let input = view.read_with(cx, |this, _| this.input.clone());
    cx.update(|window, cx| input.update(cx, |input, cx| input.set_value(text, window, cx)));
    cx.update(|window, cx| window.render_frame(cx));
}

fn value(view: &Entity<Search>, cx: &mut VisualTestContext) -> String {
    view.read_with(cx, |this, cx| this.input.read(cx).value().to_string())
}

fn listed(id: &str, cx: &mut VisualTestContext) -> bool {
    let id = id.to_owned();
    cx.update(|window, cx| {
        window.render_frame(cx);
        window.try_find(id).is_some_and(|found| found.visible())
    })
}

#[gpui_kit::test]
fn a_search_field_shows_its_clear_button_only_with_text(cx: &mut TestAppContext) {
    let (view, cx) = search(cx);
    assert!(
        !listed("find-clear", cx),
        "an empty field has no clear button"
    );
    assert_eq!(
        cx.update(|window, _| window.find("find").label().map(str::to_owned)),
        Some("Search items".to_owned()),
        "the label is the accessible name"
    );

    type_in(&view, "brow", cx);
    advance(cx, PANEL / 3);
    let part = seen("find-clear").expect("the clear button was drawn");
    assert!(0.0 < part && part < 1.0, "it unfolds, not a jump: {part}");
    advance(cx, PANEL * 2);
    assert_eq!(seen("find-clear"), Some(1.0));
    assert!(listed("find-clear", cx));
    assert_eq!(pending_frames(cx), 0, "and it stops asking for frames");

    type_in(&view, "", cx);
    advance(cx, PANEL / 3);
    assert!(listed("find-clear", cx), "it is still folding away");
    let part = seen("find-clear").unwrap();
    assert!(0.0 < part && part < 1.0, "it folds, not a jump: {part}");
    advance(cx, PANEL * 2);
    assert!(
        !listed("find-clear", cx),
        "and it is gone once it has folded"
    );
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn the_clear_button_empties_the_search_field(cx: &mut TestAppContext) {
    let (view, cx) = search(cx);
    type_in(&view, "brow", cx);
    advance(cx, PANEL * 2);
    cx.update(|window, cx| window.click("find-clear", cx));
    assert_eq!(value(&view, cx), "");
    advance(cx, PANEL * 2);
    assert!(!listed("find-clear", cx));
}

#[gpui_kit::test]
fn escape_clears_the_search_field(cx: &mut TestAppContext) {
    let (view, cx) = search(cx);
    type_in(&view, "brow", cx);
    let input = view.read_with(cx, |this, _| this.input.clone());
    cx.update(|window, cx| input.update(cx, |input, cx| input.focus(window, cx)));
    cx.update(|window, cx| window.press("escape", cx));
    assert_eq!(value(&view, cx), "");
}

struct Chips {
    chosen: usize,
    picked: Rc<RefCell<Vec<usize>>>,
    counts: [usize; 3],
}

impl Render for Chips {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (this, picked) = (cx.entity().downgrade(), self.picked.clone());
        div().w(px(400.)).p(px(12.)).child(
            FilterChips::new(
                "show",
                "Show",
                [
                    FilterChip::new("all", "All", self.counts[0]),
                    FilterChip::new("open", "Open", self.counts[1]),
                    FilterChip::new("done", "Done", self.counts[2]),
                ],
                self.chosen,
            )
            .on_choose(move |index, _, cx| {
                picked.borrow_mut().push(index);
                this.update(cx, |this, cx| {
                    this.chosen = index;
                    cx.notify();
                })
                .ok();
            }),
        )
    }
}

fn chips(chosen: usize, cx: &mut TestAppContext) -> (Entity<Chips>, &mut VisualTestContext) {
    init(cx);
    let (view, cx) = cx.add_window_view(move |_, _| Chips {
        chosen,
        picked: Rc::new(RefCell::new(Vec::new())),
        counts: [35, 1, 0],
    });
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

fn label_of(id: &str, cx: &mut VisualTestContext) -> Option<String> {
    let id = id.to_owned();
    cx.update(|window, _| window.find(id).label().map(str::to_owned))
}

#[gpui_kit::test]
fn a_filter_chip_shows_its_count(cx: &mut TestAppContext) {
    let (_, cx) = chips(0, cx);
    assert_eq!(label_of("show-all", cx).as_deref(), Some("All 35"));
    assert_eq!(label_of("show-open", cx).as_deref(), Some("Open 1"));
    assert_eq!(
        label_of("show-done", cx).as_deref(),
        Some("Done 0"),
        "a chip whose count is 0 stays shown"
    );
    assert_eq!(label_of("show", cx).as_deref(), Some("Show"));
}

#[gpui_kit::test]
fn a_filter_chip_clicked_goes_to_it_and_its_fill_moves_over_panel(cx: &mut TestAppContext) {
    let (view, cx) = chips(0, cx);
    assert_eq!(seen("show-all"), Some(1.0));
    assert_eq!(seen("show-open"), Some(0.0));
    cx.update(|window, cx| window.click("show-open", cx));
    assert_eq!(
        view.read_with(cx, |this, _| this.picked.borrow().clone()),
        [1]
    );
    advance(cx, PANEL / 3);
    let (open, all) = (seen("show-open").unwrap(), seen("show-all").unwrap());
    assert!(0.0 < open && open < 1.0, "the chosen fill moves in: {open}");
    assert!(
        0.0 < all && all < 1.0,
        "and moves out of the old one: {all}"
    );
    advance(cx, PANEL * 2);
    assert_eq!(seen("show-open"), Some(1.0));
    assert_eq!(seen("show-all"), Some(0.0));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_filter_chip_clicked_again_goes_back_to_all(cx: &mut TestAppContext) {
    let (view, cx) = chips(1, cx);
    cx.update(|window, cx| window.click("show-open", cx));
    assert_eq!(view.read_with(cx, |this, _| this.chosen), 0);
    assert_eq!(
        view.read_with(cx, |this, _| this.picked.borrow().clone()),
        [0]
    );
    cx.update(|window, cx| window.click("show-all", cx));
    assert_eq!(
        view.read_with(cx, |this, _| this.picked.borrow().clone()),
        [0],
        "the first chip clicked while chosen changes nothing"
    );
}

#[gpui_kit::test]
fn a_filter_chip_fades_its_hover_over_hover(cx: &mut TestAppContext) {
    let (_, cx) = chips(0, cx);
    cx.update(|window, cx| window.hover("show-open", cx));
    advance(cx, HOVER / 3);
    assert!(pending_frames(cx) > 0, "the hover is still moving");
    advance(cx, HOVER * 2);
    assert_eq!(pending_frames(cx), 0, "it has moved within HOVER");
}

struct Boxes {
    state: CheckState,
    next: Rc<RefCell<Vec<CheckState>>>,
}

impl Render for Boxes {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let next = self.next.clone();
        div().w(px(300.)).p(px(12.)).child(
            Checkbox::new("pick", self.state, "Select all")
                .labelled()
                .on_change(move |to, _, _| next.borrow_mut().push(to)),
        )
    }
}

fn boxes(state: CheckState, cx: &mut TestAppContext) -> (Entity<Boxes>, &mut VisualTestContext) {
    init(cx);
    let (view, cx) = cx.add_window_view(move |_, _| Boxes {
        state,
        next: Rc::new(RefCell::new(Vec::new())),
    });
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

fn turn(view: &Entity<Boxes>, state: CheckState, cx: &mut VisualTestContext) {
    cx.update(|_, cx| {
        view.update(cx, |this, cx| {
            this.state = state;
            cx.notify();
        })
    });
    cx.update(|window, cx| window.render_frame(cx));
}

#[gpui_kit::test]
fn a_checkbox_goes_from_mixed_to_checked(cx: &mut TestAppContext) {
    let (view, cx) = boxes(CheckState::Mixed, cx);
    cx.update(|window, cx| window.click("pick", cx));
    cx.update(|window, cx| window.click("pick", cx));
    assert_eq!(
        view.read_with(cx, |this, _| this.next.borrow().clone()),
        [CheckState::Checked, CheckState::Checked],
        "a mixed box asks for checked"
    );
    turn(&view, CheckState::Checked, cx);
    cx.update(|window, cx| window.click("pick", cx));
    turn(&view, CheckState::Empty, cx);
    cx.update(|window, cx| window.click("pick", cx));
    assert_eq!(
        view.read_with(cx, |this, _| this.next.borrow()[2..].to_vec()),
        [CheckState::Empty, CheckState::Checked],
        "a checked box asks for empty, an empty one for checked"
    );
    assert_eq!(label_of("pick", cx).as_deref(), Some("Select all"));
    cx.update(|window, _| {
        assert_eq!(window.find("pick").selected(), Some(false));
        assert_eq!(window.find("pick").value(), None);
    });
}

#[gpui_kit::test]
fn a_checkbox_says_it_is_mixed_and_when_it_is_checked(cx: &mut TestAppContext) {
    let (view, cx) = boxes(CheckState::Mixed, cx);
    cx.update(|window, _| {
        assert_eq!(window.find("pick").value(), Some("mixed"));
        assert_eq!(window.find("pick").selected(), Some(false));
    });
    turn(&view, CheckState::Checked, cx);
    cx.update(|window, _| {
        assert_eq!(window.find("pick").value(), None);
        assert_eq!(window.find("pick").selected(), Some(true));
    });
}

#[gpui_kit::test]
fn a_checkbox_mark_animates_in(cx: &mut TestAppContext) {
    let (view, cx) = boxes(CheckState::Empty, cx);
    assert_eq!(seen("pick-mark"), Some(0.0), "empty: no mark");
    assert_eq!(pending_frames(cx), 0);
    turn(&view, CheckState::Checked, cx);
    advance(cx, HOVER / 3);
    let part = seen("pick-mark").unwrap();
    assert!(0.0 < part && part < 1.0, "the mark draws in: {part}");
    advance(cx, HOVER * 2);
    assert_eq!(seen("pick-mark"), Some(1.0));
    assert_eq!(pending_frames(cx), 0, "and it stops asking for frames");
    turn(&view, CheckState::Empty, cx);
    advance(cx, HOVER / 3);
    let part = seen("pick-mark").unwrap();
    assert!(0.0 < part && part < 1.0, "and draws out: {part}");
    advance(cx, HOVER * 2);
    assert_eq!(seen("pick-mark"), Some(0.0));
}

#[gpui_kit::test]
fn a_checkbox_first_drawn_checked_is_simply_checked(cx: &mut TestAppContext) {
    let (_, cx) = boxes(CheckState::Checked, cx);
    assert_eq!(seen("pick-mark"), Some(1.0));
    assert_eq!(pending_frames(cx), 0);
}

struct Pills {
    chosen: usize,
    picked: Rc<Cell<usize>>,
}

const SETTINGS: [(&str, &str, &str, Tone); 3] = [
    ("off", "Off", "Never offered.", Tone::Outline),
    ("on", "On", "Always offered.", Tone::Ok),
    ("ask", "Ask", "Offered after a question.", Tone::Strong),
];

impl Render for Pills {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let picked = self.picked.clone();
        div().w(px(400.)).p(px(12.)).child(
            Select::new(
                "mode",
                SETTINGS.map(|(key, title, line, tone)| {
                    SelectOption::new(key, title).line(line).tone(tone)
                }),
                self.chosen,
            )
            .compact()
            .width(px(86.))
            .on_choose(move |index, _, _| picked.set(index)),
        )
    }
}

fn pills(chosen: usize, cx: &mut TestAppContext) -> (Entity<Pills>, &mut VisualTestContext) {
    init(cx);
    let (view, cx) = cx.add_window_view(move |_, _| Pills {
        chosen,
        picked: Rc::new(Cell::new(usize::MAX)),
    });
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

#[gpui_kit::test]
fn a_compact_select_is_24_px_high(cx: &mut TestAppContext) {
    let (_, cx) = pills(0, cx);
    let bounds = cx.update(|window, _| window.find("mode").bounds());
    assert_eq!(bounds.size.height, px(24.));
    assert_eq!(bounds.size.width, px(86.));
    assert_eq!(label_of("mode", cx).as_deref(), Some("Off"));
}

#[gpui_kit::test]
fn a_compact_select_menu_shows_each_option_sentence(cx: &mut TestAppContext) {
    let (view, cx) = pills(0, cx);
    cx.update(|window, cx| window.click("mode", cx));
    advance(cx, ENTER * 2);
    for (key, _, line, _) in SETTINGS {
        assert_eq!(
            label_of(&format!("mode-{key}-line"), cx).as_deref(),
            Some(line)
        );
    }
    cx.update(|window, cx| window.click("mode-ask", cx));
    assert_eq!(view.read_with(cx, |this, _| this.picked.get()), 2);
    advance(cx, ENTER * 2);
    assert!(!listed("mode-menu", cx), "picking closes it");
}

#[gpui_kit::test]
fn a_compact_select_takes_the_tone_of_its_value(cx: &mut TestAppContext) {
    let (view, cx) = pills(1, cx);
    let p = cx.update(|_, cx| palette(cx));
    assert_eq!(colour("mode-pill"), Some(Tone::Ok.fill(&p)));
    assert_ne!(Tone::Ok.fill(&p), Tone::Strong.fill(&p));
    assert_eq!(pending_frames(cx), 0);

    cx.update(|_, cx| {
        view.update(cx, |this, cx| {
            this.chosen = 2;
            cx.notify();
        })
    });
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, PANEL / 3);
    let part = colour("mode-pill").unwrap();
    assert_ne!(part, Tone::Ok.fill(&p), "the fill has left the old tone");
    assert_ne!(part, Tone::Strong.fill(&p), "and is not at the new one yet");
    advance(cx, PANEL * 2);
    assert_eq!(colour("mode-pill"), Some(Tone::Strong.fill(&p)));
    assert_eq!(pending_frames(cx), 0);
}
