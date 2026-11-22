//! `ModelPicker`, one control for the provider, its model and its effort, on GPUI's test platform:
//! closed it says the three in one label; open, one menu with the providers, the models of the
//! chosen one, then the efforts of the chosen model, none when it offers none: three columns side
//! by side, the last one folding away when the model offers no effort.

mod common;

use std::cell::RefCell;
use std::rc::Rc;

use common::{advance, init, pending_frames};
use gpui_design_system::motion::{ENTER, HOVER, PANEL};
use gpui_design_system::seen::seen;
use gpui_design_system::{Chosen, ModelEntry, ModelPicker};
use gpui_kit::test::{TestQueryExt as _, TestWindowExt};
use gpui_kit::{
    Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled, TestAppContext,
    VisualTestContext, Window, div, px, size,
};

/// What the picker reported: what was chosen, and its effort.
type Seen = Rc<RefCell<Vec<(Chosen, Option<SharedString>)>>>;

struct Host {
    /// In the bottom right corner of the window, as a composer's dock has it.
    corner: bool,
    chosen: Chosen,
    effort: Option<SharedString>,
    seen: Seen,
}

fn models() -> Vec<ModelEntry> {
    vec![
        ModelEntry::new("aster", "Aster", Some("lumen-2".into()), "Lumen 2")
            .efforts(["low", "high"])
            .proposes("high"),
        ModelEntry::new("aster", "Aster", Some("spark".into()), "Spark"),
        ModelEntry::new("borealis", "Borealis", Some("gamma".into()), "Gamma")
            .efforts(["low", "medium", "high"])
            .proposes("medium"),
        ModelEntry::new("borealis", "Borealis", Some("delta".into()), "Delta"),
        ModelEntry::new("cinder", "Cinder", Some("ember".into()), "Ember"),
    ]
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (this, seen) = (cx.entity().downgrade(), self.seen.clone());
        let picker = ModelPicker::new("picker", self.chosen.clone())
            .models(models())
            .effort(self.effort.clone())
            .on_choose(move |chosen, effort, _, cx| {
                seen.borrow_mut().push((chosen.clone(), effort.clone()));
                this.update(cx, |this, cx| {
                    this.chosen = chosen;
                    this.effort = effort;
                    cx.notify();
                })
                .ok();
            });
        if self.corner {
            return div()
                .size_full()
                .p(px(16.))
                .flex()
                .flex_col()
                .justify_end()
                .items_end()
                .child(picker);
        }
        div().size_full().p(px(16.)).child(picker)
    }
}

fn host(cx: &mut TestAppContext, chosen: Chosen) -> (Entity<Host>, &mut VisualTestContext, Seen) {
    placed(cx, chosen, false)
}

fn placed(
    cx: &mut TestAppContext,
    chosen: Chosen,
    corner: bool,
) -> (Entity<Host>, &mut VisualTestContext, Seen) {
    init(cx);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let given = seen.clone();
    let (view, cx) = cx.add_window_view(move |_, _| Host {
        corner,
        chosen,
        effort: None,
        seen: given,
    });
    cx.simulate_resize(size(px(900.), px(700.)));
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx, seen)
}

fn aster() -> Chosen {
    Chosen::Model("aster".into(), Some("lumen-2".into()))
}

fn shown(id: &str, cx: &mut VisualTestContext) -> bool {
    let id = id.to_owned();
    cx.update(|window, cx| {
        window.render_frame(cx);
        window.try_find(id).is_some_and(|found| found.visible())
    })
}

fn label(id: &str, cx: &mut VisualTestContext) -> Option<String> {
    let id = id.to_owned();
    cx.update(|window, _| {
        window
            .try_find(id)
            .and_then(|found| found.label().map(str::to_owned))
    })
}

fn selected(id: &str, cx: &mut VisualTestContext) -> Option<bool> {
    let id = id.to_owned();
    cx.update(|window, _| window.try_find(id).and_then(|found| found.selected()))
}

fn open(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.click("picker", cx));
    advance(cx, ENTER * 2);
}

fn click(id: &str, cx: &mut VisualTestContext) {
    let id = id.to_owned();
    cx.update(|window, cx| window.click(id, cx));
    advance(cx, PANEL * 2);
}

#[gpui_kit::test]
fn the_closed_picker_says_the_provider_the_model_and_the_effort(cx: &mut TestAppContext) {
    let (_, cx, _) = host(cx, aster());
    assert_eq!(
        label("picker", cx).as_deref(),
        Some("Aster · Lumen 2 · high"),
        "the effort that applies is said when none is chosen"
    );
    assert!(!shown("picker-menu", cx), "closed: no menu");
}

#[gpui_kit::test]
fn the_open_menu_lists_the_providers_then_the_models_then_the_efforts(cx: &mut TestAppContext) {
    let (_, cx, _) = host(cx, aster());
    open(cx);
    assert!(shown("picker-menu", cx));
    assert_eq!(label("picker-provider-aster", cx).as_deref(), Some("Aster"));
    assert_eq!(
        label("picker-provider-borealis", cx).as_deref(),
        Some("Borealis")
    );
    assert_eq!(selected("picker-provider-aster", cx), Some(true));
    assert_eq!(selected("picker-provider-borealis", cx), Some(false));
    // The models of the chosen provider only.
    assert_eq!(
        label("picker-model-aster-lumen-2", cx).as_deref(),
        Some("Lumen 2")
    );
    assert_eq!(
        label("picker-model-aster-spark", cx).as_deref(),
        Some("Spark")
    );
    assert!(label("picker-model-borealis-gamma", cx).is_none());
    assert_eq!(selected("picker-model-aster-lumen-2", cx), Some(true));
    assert_eq!(selected("picker-model-aster-spark", cx), Some(false));
    // The efforts of the chosen model, the one that applies selected, none named "default".
    assert_eq!(label("picker-effort-low", cx).as_deref(), Some("low"));
    assert_eq!(label("picker-effort-high", cx).as_deref(), Some("high"));
    assert_eq!(selected("picker-effort-high", cx), Some(true));
    assert_eq!(selected("picker-effort-low", cx), Some(false));
    cx.update(|window, _| assert!(window.try_find_by_label("Default").is_none()));
}

#[gpui_kit::test]
fn choosing_a_provider_picks_its_first_model_and_that_model_s_default_effort(
    cx: &mut TestAppContext,
) {
    let (_, cx, seen) = host(cx, aster());
    open(cx);
    click("picker-provider-borealis", cx);
    assert_eq!(
        seen.borrow().as_slice(),
        [(Chosen::Model("borealis".into(), Some("gamma".into())), None)],
        "its first model, with the effort that model proposes (reported as none)"
    );
    assert_eq!(
        label("picker", cx).as_deref(),
        Some("Borealis · Gamma · medium")
    );
    // The menu stays open on the models of the new provider and the efforts of its first model.
    assert!(shown("picker-menu", cx));
    assert_eq!(selected("picker-provider-borealis", cx), Some(true));
    assert_eq!(selected("picker-model-borealis-gamma", cx), Some(true));
    assert!(label("picker-model-aster-spark", cx).is_none());
    assert_eq!(label("picker-effort-medium", cx).as_deref(), Some("medium"));
    assert_eq!(selected("picker-effort-medium", cx), Some(true));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn choosing_the_chosen_provider_again_reports_nothing(cx: &mut TestAppContext) {
    let (_, cx, seen) = host(cx, aster());
    open(cx);
    click("picker-provider-aster", cx);
    assert!(seen.borrow().is_empty());
}

#[gpui_kit::test]
fn a_model_without_effort_shows_none(cx: &mut TestAppContext) {
    let (_, cx, seen) = host(cx, aster());
    open(cx);
    // The provider's first model offers none: the effort column folds away with its title.
    let width = |cx: &mut VisualTestContext| {
        cx.update(|window, _| window.find("picker-efforts").bounds().size.width)
    };
    let whole = width(cx);
    cx.update(|window, cx| window.click("picker-provider-cinder", cx));
    assert_eq!(
        seen.borrow().last(),
        Some(&(Chosen::Model("cinder".into(), Some("ember".into())), None))
    );
    advance(cx, PANEL / 3);
    let midway = width(cx);
    assert!(
        px(0.) < midway && midway < whole,
        "it folds, not vanishes: {midway:?} of {whole:?}"
    );
    advance(cx, PANEL * 2);
    assert!(!shown("picker-effort-high", cx));
    assert!(!shown("picker-effort-low", cx));
    assert!(!shown("picker-efforts-label", cx));
    assert_eq!(label("picker", cx).as_deref(), Some("Cinder · Ember"));
    assert!(
        shown("picker-menu", cx),
        "a provider pick leaves the menu open"
    );
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_model_that_offers_no_effort_ends_the_choice(cx: &mut TestAppContext) {
    let (_, cx, seen) = host(cx, aster());
    open(cx);
    cx.update(|window, cx| window.click("picker-model-aster-spark", cx));
    assert_eq!(
        seen.borrow().last(),
        Some(&(Chosen::Model("aster".into(), Some("spark".into())), None))
    );
    advance(cx, ENTER * 2);
    assert!(
        !shown("picker-menu", cx),
        "nothing is left to choose: it closed"
    );
    assert_eq!(label("picker", cx).as_deref(), Some("Aster · Spark"));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn the_efforts_unfold_when_a_model_that_offers_them_is_chosen(cx: &mut TestAppContext) {
    let (_, cx, _) = host(cx, Chosen::Model("aster".into(), Some("spark".into())));
    open(cx);
    assert!(!shown("picker-effort-high", cx));
    cx.update(|window, cx| window.click("picker-provider-borealis", cx));
    advance(cx, PANEL / 3);
    let midway = cx.update(|window, _| window.find("picker-efforts").bounds().size.width);
    advance(cx, PANEL * 2);
    let whole = cx.update(|window, _| window.find("picker-efforts").bounds().size.width);
    assert!(midway < whole, "it unfolds: {midway:?} < {whole:?}");
    assert!(shown("picker-effort-high", cx));
    click("picker-model-borealis-delta", cx);
    open(cx);
    // Delta has none, and its menu closed: open again, still none.
    assert!(!shown("picker-effort-medium", cx));
    assert!(shown("picker-model-borealis-gamma", cx));
}

#[gpui_kit::test]
fn choosing_an_effort_reports_it_and_closes_the_menu(cx: &mut TestAppContext) {
    let (_, cx, seen) = host(cx, aster());
    open(cx);
    click("picker-effort-low", cx);
    assert_eq!(
        seen.borrow().last(),
        Some(&(aster(), Some("low".into()))),
        "the model chosen, with the effort"
    );
    assert_eq!(
        label("picker", cx).as_deref(),
        Some("Aster · Lumen 2 · low")
    );
    assert!(
        !shown("picker-menu", cx),
        "the last section ends the choice"
    );
    // The effort the model proposes is no effort chosen.
    open(cx);
    assert_eq!(selected("picker-effort-low", cx), Some(true));
    click("picker-effort-high", cx);
    assert_eq!(seen.borrow().last(), Some(&(aster(), None)));
    assert_eq!(
        label("picker", cx).as_deref(),
        Some("Aster · Lumen 2 · high")
    );
}

#[gpui_kit::test]
fn choosing_another_model_of_the_provider_resets_the_effort(cx: &mut TestAppContext) {
    let (view, cx, seen) = host(cx, aster());
    view.update(cx, |host, cx| {
        host.effort = Some("low".into());
        cx.notify();
    });
    cx.update(|window, cx| window.render_frame(cx));
    open(cx);
    click("picker-provider-borealis", cx);
    assert_eq!(
        seen.borrow().last(),
        Some(&(Chosen::Model("borealis".into(), Some("gamma".into())), None)),
        "the effort of the other provider's model does not follow"
    );
}

#[gpui_kit::test]
fn the_label_that_changes_fades_over_hover(cx: &mut TestAppContext) {
    let (_, cx, _) = host(cx, aster());
    assert_eq!(
        seen("picker-label"),
        Some(1.0),
        "a first label is simply there"
    );
    open(cx);
    cx.update(|window, cx| window.click("picker-provider-borealis", cx));
    advance(cx, HOVER / 3);
    let midway = seen("picker-label").unwrap();
    assert!(midway < 1.0, "the new label is fading in: {midway}");
    advance(cx, HOVER * 2);
    assert_eq!(seen("picker-label"), Some(1.0));
    assert_eq!(
        label("picker", cx).as_deref(),
        Some("Borealis · Gamma · medium")
    );
}

#[gpui_kit::test]
fn an_entry_fades_its_hover_in_over_hover(cx: &mut TestAppContext) {
    let (_, cx, _) = host(cx, aster());
    open(cx);
    assert_eq!(seen("picker-provider-borealis-hover"), Some(0.0));
    cx.update(|window, cx| window.hover("picker-provider-borealis", cx));
    advance(cx, HOVER / 3);
    let midway = seen("picker-provider-borealis-hover").unwrap();
    assert!(0.0 < midway && midway < 1.0, "{midway}");
    advance(cx, HOVER * 2);
    assert_eq!(seen("picker-provider-borealis-hover"), Some(1.0));
}

#[gpui_kit::test]
fn the_chosen_mark_moves_over_panel(cx: &mut TestAppContext) {
    let (_, cx, _) = host(cx, aster());
    open(cx);
    assert_eq!(seen("picker-provider-aster-mark"), Some(1.0));
    assert_eq!(seen("picker-provider-borealis-mark"), Some(0.0));
    cx.update(|window, cx| window.click("picker-provider-borealis", cx));
    advance(cx, PANEL / 3);
    let (leaving, arriving) = (
        seen("picker-provider-aster-mark").unwrap(),
        seen("picker-provider-borealis-mark").unwrap(),
    );
    assert!(leaving < 1.0 && leaving > 0.0, "{leaving}");
    assert!(arriving > 0.0 && arriving < 1.0, "{arriving}");
    advance(cx, PANEL * 2);
    assert_eq!(seen("picker-provider-aster-mark"), Some(0.0));
    assert_eq!(seen("picker-provider-borealis-mark"), Some(1.0));
}

#[gpui_kit::test]
fn the_menu_opens_and_closes_by_fading(cx: &mut TestAppContext) {
    let (_, cx, _) = host(cx, aster());
    cx.update(|window, cx| window.click("picker", cx));
    advance(cx, ENTER / 3);
    assert!(shown("picker-menu", cx), "it fades in, not at once");
    advance(cx, ENTER * 2);
    cx.update(|window, cx| window.click("picker", cx));
    advance(cx, HOVER / 3);
    assert!(shown("picker-menu", cx), "it fades out, not at once");
    advance(cx, HOVER * 2);
    assert!(!shown("picker-menu", cx));
    assert_eq!(pending_frames(cx), 0);
}

fn bounds(id: &str, cx: &mut VisualTestContext) -> gpui_kit::Bounds<gpui_kit::Pixels> {
    let id = id.to_owned();
    cx.update(|window, _| window.find(id).bounds())
}

#[gpui_kit::test]
fn the_menu_is_three_columns_side_by_side_each_with_its_heading(cx: &mut TestAppContext) {
    let (_, cx, _) = host(cx, aster());
    open(cx);
    for (id, title) in [
        ("picker-providers-label", "Provider"),
        ("picker-models-label", "Model"),
        ("picker-efforts-label", "Effort"),
    ] {
        assert_eq!(label(id, cx).as_deref(), Some(title));
    }
    let (providers, models, efforts) = (
        bounds("picker-providers-label", cx),
        bounds("picker-models-label", cx),
        bounds("picker-efforts-label", cx),
    );
    assert!(
        providers.right() <= models.left() && models.right() <= efforts.left(),
        "left to right: {providers:?} {models:?} {efforts:?}"
    );
    assert_eq!(providers.top(), models.top(), "side by side, not stacked");
    assert_eq!(models.top(), efforts.top());
    // Each entry sits in the column of its title, under it.
    for (entry, heading) in [
        ("picker-provider-borealis", providers),
        ("picker-model-aster-spark", models),
        ("picker-effort-low", efforts),
    ] {
        let at = bounds(entry, cx);
        assert!(
            at.left() >= heading.left() - px(12.) && at.top() >= heading.bottom(),
            "{entry} under its heading: {at:?} {heading:?}"
        );
        assert!(at.left() < heading.right() + px(150.), "{entry}: {at:?}");
    }
    // Shorter than a stack of the three: no entry is below the tallest column's own rows.
    let menu = bounds("picker-menu", cx);
    assert!(menu.size.height < px(200.), "{menu:?}");
}

#[gpui_kit::test]
fn the_menu_does_not_move_when_the_effort_column_folds_and_comes_back(cx: &mut TestAppContext) {
    let (_, cx, _) = host(cx, aster());
    open(cx);
    let at = bounds("picker-providers-label", cx);
    let menu = bounds("picker-menu", cx);
    click("picker-provider-cinder", cx);
    assert!(!shown("picker-effort-high", cx));
    assert_eq!(
        bounds("picker-providers-label", cx),
        at,
        "the columns that stay do not move"
    );
    let narrow = bounds("picker-menu", cx);
    assert!(narrow.size.width < menu.size.width, "{narrow:?} {menu:?}");
    assert_eq!(narrow.left(), menu.left());
    click("picker-provider-borealis", cx);
    assert!(shown("picker-effort-high", cx));
    assert_eq!(bounds("picker-providers-label", cx), at);
    assert_eq!(bounds("picker-menu", cx), menu, "it is back as it was");
}

#[gpui_kit::test]
fn the_models_column_fades_when_the_provider_changes(cx: &mut TestAppContext) {
    let (_, cx, _) = host(cx, aster());
    open(cx);
    assert_eq!(seen("picker-models-fade"), Some(1.0));
    cx.update(|window, cx| window.click("picker-provider-borealis", cx));
    advance(cx, HOVER / 3);
    let midway = seen("picker-models-fade").unwrap();
    assert!(midway < 1.0, "the new models fade in: {midway}");
    advance(cx, HOVER * 2);
    assert_eq!(seen("picker-models-fade"), Some(1.0));
}

#[gpui_kit::test]
fn the_menu_never_leaves_the_window(cx: &mut TestAppContext) {
    // In the corner of the window, as the Reply dock has it: it opens upward, and lines up with
    // the button's right edge, though the menu is wider than the button.
    let (_, cx, _) = placed(cx, aster(), true);
    open(cx);
    let (menu, button, viewport) = cx.update(|window, _| {
        (
            window.find("picker-menu").bounds(),
            window.find("picker").bounds(),
            window.viewport_size(),
        )
    });
    assert!(
        menu.left() >= px(0.) && menu.right() <= viewport.width,
        "{menu:?} in {viewport:?}"
    );
    assert!(
        menu.top() >= px(0.) && menu.bottom() <= viewport.height,
        "{menu:?} in {viewport:?}"
    );
    assert!(menu.bottom() <= button.top(), "upward: {menu:?} {button:?}");
    assert_eq!(menu.right(), button.right(), "on the button's right edge");
    // Still so with the effort column folded: it was placed with its widest size.
    click("picker-provider-cinder", cx);
    let folded = bounds("picker-menu", cx);
    assert!(folded.right() <= viewport.width && folded.bottom() <= button.top());
    assert_eq!(folded.right(), button.right());
}

#[gpui_kit::test]
fn a_window_narrower_than_the_menu_keeps_it_inside(cx: &mut TestAppContext) {
    let (_, cx, _) = host(cx, aster());
    cx.simulate_resize(size(px(400.), px(500.)));
    cx.update(|window, cx| window.render_frame(cx));
    open(cx);
    let (menu, viewport) =
        cx.update(|window, _| (window.find("picker-menu").bounds(), window.viewport_size()));
    assert!(
        menu.left() >= px(0.) && menu.right() <= viewport.width,
        "{menu:?} in {viewport:?}"
    );
}
