//! `TreeRow`: a row with nothing to fold keeps its chevron's place and fades the chevron in or out
//! over `PANEL` where it stands, never a pop and never a shift of the title; what the row draws in
//! its leading slot is handed the row's current fill, so that a child can sit on it; a title said
//! faint is drawn in the faint ink, which fades in and out over `HOVER` and which hover and choice
//! still lift. On GPUI's test platform.

mod common;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use common::{advance, init, pending_frames};
use gpui_design_system::color::over;
use gpui_design_system::motion::{HOVER, PANEL};
use gpui_design_system::seen::{colour, seen};
use gpui_design_system::theme::palette;
use gpui_design_system::{Icon, MachineMark, MachineState, TreeRow, toggle_id};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Bounds, Context, Entity, Hsla, IntoElement, ParentElement, Pixels, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, point, px,
};

/// What a row was asked to draw and what a click reached.
#[derive(Default, Clone)]
struct Reached {
    folded: Rc<Cell<usize>>,
    opened: Rc<Cell<usize>>,
    fill: Rc<RefCell<Option<Hsla>>>,
}

struct Tree {
    foldable: bool,
    selected: bool,
    faint: bool,
    reached: Reached,
}

impl Render for Tree {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (folded, opened, fill) = (
            self.reached.folded.clone(),
            self.reached.opened.clone(),
            self.reached.fill.clone(),
        );
        div().w(px(240.)).child(
            TreeRow::new("tree-row", "atlas")
                .top()
                .expanded(true)
                .foldable(self.foldable)
                .selected(self.selected)
                .faint(self.faint)
                .on_toggle(move |_, _, _| folded.set(folded.get() + 1))
                .on_click(move |_, _, _| opened.set(opened.get() + 1))
                .leading_with(move |on| {
                    *fill.borrow_mut() = Some(on);
                    MachineMark::new(
                        "tree-mark",
                        Icon::Lucide("server"),
                        gpui_design_system::color::hex(0x2dd4bf),
                        MachineState::Online,
                        "Workstation",
                    )
                    .on(on)
                }),
        )
    }
}

fn tree(
    foldable: bool,
    reduce: bool,
    cx: &mut TestAppContext,
) -> (Entity<Tree>, Reached, &mut VisualTestContext) {
    start(foldable, false, reduce, cx)
}

fn start(
    foldable: bool,
    faint: bool,
    reduce: bool,
    cx: &mut TestAppContext,
) -> (Entity<Tree>, Reached, &mut VisualTestContext) {
    init(cx);
    cx.update(|cx| cx.set_reduce_motion(reduce));
    let reached = Reached::default();
    let shared = reached.clone();
    let (view, cx) = cx.add_window_view(move |_, _| Tree {
        foldable,
        selected: false,
        faint,
        reached: shared,
    });
    // The pointer rests away from the row, so that none is hovered that did not ask to be.
    cx.simulate_mouse_move(point(px(600.), px(400.)), None, Default::default());
    cx.update(|window, cx| window.render_frame(cx));
    (view, reached, cx)
}

fn change(view: &Entity<Tree>, cx: &mut VisualTestContext, f: impl FnOnce(&mut Tree)) {
    cx.update(|_, cx| {
        view.update(cx, |tree, cx| {
            f(tree);
            cx.notify();
        })
    });
    cx.update(|window, cx| window.render_frame(cx));
}

fn bounds(id: &str, cx: &mut VisualTestContext) -> Bounds<Pixels> {
    cx.update(|window, _| window.find(id.to_string()).bounds())
}

fn chevron(cx: &mut VisualTestContext) -> f32 {
    cx.update(|window, cx| window.render_frame(cx));
    seen("tree-row-chevron").expect("the chevron notes how visible it is")
}

/// Two colours are the same to the eye.
fn same(a: Hsla, b: Hsla) -> bool {
    [(a.h, b.h), (a.s, b.s), (a.l, b.l), (a.a, b.a)]
        .iter()
        .all(|(x, y)| (x - y).abs() < 1e-3)
}

#[gpui_kit::test]
fn a_row_that_gains_something_to_fold_fades_its_chevron_in_where_it_stands(
    cx: &mut TestAppContext,
) {
    let (view, _, cx) = tree(false, false, cx);
    assert_eq!(chevron(cx), 0.0, "nothing to fold, nothing drawn");
    advance(cx, PANEL * 2);
    assert_eq!(pending_frames(cx), 0, "a first state is simply there");
    let (title, toggle) = (bounds("tree-row-title", cx), bounds_of_toggle(cx));

    change(&view, cx, |tree| tree.foldable = true);
    assert!(pending_frames(cx) > 0, "the chevron fades in");
    advance(cx, PANEL / 3);
    let partway = chevron(cx);
    assert!(0.0 < partway && partway < 1.0, "part way: {partway}");
    assert_eq!(
        bounds("tree-row-title", cx),
        title,
        "the title never shifts"
    );
    assert_eq!(bounds_of_toggle(cx), toggle, "the chevron never moves");

    advance(cx, PANEL * 2);
    assert_eq!(chevron(cx), 1.0);
    assert_eq!(bounds("tree-row-title", cx), title);
    assert_eq!(bounds_of_toggle(cx), toggle);
    assert_eq!(pending_frames(cx), 0, "nothing is drawn once it arrived");
}

#[gpui_kit::test]
fn a_row_left_with_nothing_to_fold_fades_its_chevron_out_where_it_stands(cx: &mut TestAppContext) {
    let (view, _, cx) = tree(true, false, cx);
    assert_eq!(chevron(cx), 1.0);
    let (title, toggle) = (bounds("tree-row-title", cx), bounds_of_toggle(cx));

    change(&view, cx, |tree| tree.foldable = false);
    advance(cx, PANEL / 3);
    let partway = chevron(cx);
    assert!(0.0 < partway && partway < 1.0, "on its way out: {partway}");
    assert_eq!(
        bounds("tree-row-title", cx),
        title,
        "the title never shifts"
    );
    assert_eq!(bounds_of_toggle(cx), toggle);

    advance(cx, PANEL * 2);
    assert_eq!(chevron(cx), 0.0);
    assert_eq!(bounds("tree-row-title", cx), title, "its place is kept");
    assert_eq!(bounds_of_toggle(cx), toggle);
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn with_reduced_motion_a_chevron_that_comes_or_goes_jumps(cx: &mut TestAppContext) {
    let (view, _, cx) = tree(true, true, cx);
    change(&view, cx, |tree| tree.foldable = false);
    advance(cx, std::time::Duration::from_millis(16));
    assert_eq!(chevron(cx), 0.0);
    change(&view, cx, |tree| tree.foldable = true);
    advance(cx, std::time::Duration::from_millis(16));
    assert_eq!(chevron(cx), 1.0);
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_click_where_the_chevron_was_is_the_row_s_when_there_is_nothing_to_fold(
    cx: &mut TestAppContext,
) {
    let (view, reached, cx) = tree(true, false, cx);
    cx.update(|window, cx| window.click(toggle_id("tree-row"), cx));
    assert_eq!((reached.folded.get(), reached.opened.get()), (1, 0));

    change(&view, cx, |tree| tree.foldable = false);
    cx.update(|window, cx| window.click(toggle_id("tree-row"), cx));
    assert_eq!(
        (reached.folded.get(), reached.opened.get()),
        (1, 1),
        "nothing to fold: the click opens the row and folds nothing"
    );
}

#[gpui_kit::test]
fn a_row_hands_its_leading_slot_the_fill_it_is_drawn_in_and_the_fill_moves_over_hover(
    cx: &mut TestAppContext,
) {
    let (_, reached, cx) = tree(true, false, cx);
    let side = cx.update(|_, cx| palette(cx)).side;
    let handed = || reached.fill.borrow().expect("the slot was drawn");
    assert!(same(handed(), side), "at rest, the tree's own colour");

    cx.update(|window, cx| window.hover("tree-row", cx));
    advance(cx, HOVER / 3);
    let partway = handed();
    let hover = cx.update(|_, cx| palette(cx)).hover;
    assert!(!same(partway, side) && !same(partway, hover), "on its way");
    advance(cx, HOVER * 2);
    assert!(
        same(handed(), over(hover, 1.0, side)),
        "hovered: the hover fill"
    );
}

#[gpui_kit::test]
fn a_chosen_row_hands_its_leading_slot_the_chosen_fill(cx: &mut TestAppContext) {
    let (view, reached, cx) = tree(true, false, cx);
    change(&view, cx, |tree| tree.selected = true);
    advance(cx, PANEL * 2);
    let sel = cx.update(|_, cx| palette(cx)).sel;
    assert!(same(reached.fill.borrow().unwrap(), sel), "the chosen fill");
}

#[gpui_kit::test]
fn a_machine_mark_in_the_leading_slot_cuts_its_badge_out_in_the_row_s_fill(
    cx: &mut TestAppContext,
) {
    let (_, reached, cx) = tree(true, false, cx);
    cx.update(|window, cx| window.hover("tree-row", cx));
    advance(cx, HOVER * 3);
    let fill = reached.fill.borrow().unwrap();
    let cut = colour("tree-mark-cutout").expect("the mark was drawn");
    assert!(same(cut, fill), "{cut:?} is the row's fill {fill:?}");
    assert!(
        !same(cut, cx.update(|_, cx| palette(cx)).side),
        "and not the tree's colour on a hovered row"
    );
}

/// The ink the row's title was last drawn in.
fn ink(cx: &mut VisualTestContext) -> Hsla {
    cx.update(|window, cx| window.render_frame(cx));
    colour("tree-row-title").expect("the title notes its ink")
}

#[gpui_kit::test]
fn a_faint_row_draws_its_title_in_the_faint_ink(cx: &mut TestAppContext) {
    let (_, _, cx) = start(true, true, false, cx);
    let p = cx.update(|_, cx| palette(cx));
    advance(cx, PANEL * 2);
    assert_eq!(
        pending_frames(cx),
        0,
        "a row first drawn faint is simply faint"
    );
    assert!(same(ink(cx), p.faint), "a title said faint is drawn faint");
}

#[gpui_kit::test]
fn a_row_that_is_not_faint_keeps_the_ink_of_its_level(cx: &mut TestAppContext) {
    let (_, _, cx) = tree(true, false, cx);
    let p = cx.update(|_, cx| palette(cx));
    assert!(
        same(ink(cx), p.text),
        "a first-level row's title is the text ink"
    );
}

#[gpui_kit::test]
fn a_row_that_becomes_faint_fades_its_ink_over_hover(cx: &mut TestAppContext) {
    let (view, _, cx) = tree(true, false, cx);
    let p = cx.update(|_, cx| palette(cx));
    let before = ink(cx);

    change(&view, cx, |tree| tree.faint = true);
    assert!(pending_frames(cx) > 0, "the ink fades");
    advance(cx, HOVER / 3);
    let partway = ink(cx);
    assert!(
        !same(partway, before) && !same(partway, p.faint),
        "part way: {partway:?}"
    );

    advance(cx, HOVER * 2);
    assert!(same(ink(cx), p.faint), "arrived");
    assert_eq!(pending_frames(cx), 0, "nothing is drawn once it arrived");

    change(&view, cx, |tree| tree.faint = false);
    advance(cx, HOVER / 3);
    assert!(!same(ink(cx), p.faint), "and fades back");
    advance(cx, HOVER * 2);
    assert!(same(ink(cx), before));
}

#[gpui_kit::test]
fn with_reduced_motion_a_title_that_becomes_faint_jumps(cx: &mut TestAppContext) {
    let (view, _, cx) = tree(true, true, cx);
    change(&view, cx, |tree| tree.faint = true);
    advance(cx, std::time::Duration::from_millis(16));
    let p = cx.update(|_, cx| palette(cx));
    assert!(same(ink(cx), p.faint));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_faint_row_hovered_keeps_its_hover_fill(cx: &mut TestAppContext) {
    let (_, reached, cx) = start(true, true, false, cx);
    let p = cx.update(|_, cx| palette(cx));
    cx.update(|window, cx| window.hover("tree-row", cx));
    advance(cx, HOVER * 3);
    assert!(
        same(reached.fill.borrow().unwrap(), over(p.hover, 1.0, p.side)),
        "the hover fill is drawn under a faint title"
    );
    assert!(same(ink(cx), p.text), "and the pointer brings the title up");
}

#[gpui_kit::test]
fn a_faint_row_chosen_keeps_its_chosen_fill(cx: &mut TestAppContext) {
    let (view, reached, cx) = start(true, true, false, cx);
    let p = cx.update(|_, cx| palette(cx));
    change(&view, cx, |tree| tree.selected = true);
    advance(cx, PANEL * 2);
    assert!(
        same(reached.fill.borrow().unwrap(), p.sel),
        "the chosen fill"
    );
    assert!(
        same(ink(cx), p.text),
        "and the chosen title is read in full"
    );
}

fn bounds_of_toggle(cx: &mut VisualTestContext) -> Bounds<Pixels> {
    cx.update(|window, _| window.find(toggle_id("tree-row")).bounds())
}
