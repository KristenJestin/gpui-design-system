//! `TreeRow`: a row with nothing to fold keeps its chevron's place and fades the chevron in or out
//! over `PANEL` where it stands, never a pop and never a shift of the title; what the row draws in
//! its leading slot is handed the row's current fill, so that a child can sit on it. On GPUI's
//! test platform.

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
    TestAppContext, VisualTestContext, Window, div, px,
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
    init(cx);
    cx.update(|cx| cx.set_reduce_motion(reduce));
    let reached = Reached::default();
    let shared = reached.clone();
    let (view, cx) = cx.add_window_view(move |_, _| Tree {
        foldable,
        selected: false,
        reached: shared,
    });
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

fn bounds_of_toggle(cx: &mut VisualTestContext) -> Bounds<Pixels> {
    cx.update(|window, _| window.find(toggle_id("tree-row")).bounds())
}
