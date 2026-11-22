//! `ModelPicker`, one control for the provider, its model and its effort, on GPUI's test platform:
//! closed it says the three in one label; open, one menu with the providers, the models of the
//! chosen one, then the efforts of the chosen model, none when it offers none.

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
        div().size_full().p(px(16.)).child(picker)
    }
}

fn host(cx: &mut TestAppContext, chosen: Chosen) -> (Entity<Host>, &mut VisualTestContext, Seen) {
    init(cx);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let given = seen.clone();
    let (view, cx) = cx.add_window_view(move |_, _| Host {
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
    // The provider's first model offers none: its efforts fold away with their title.
    let height = |cx: &mut VisualTestContext| {
        cx.update(|window, _| window.find("picker-efforts").bounds().size.height)
    };
    let whole = height(cx);
    cx.update(|window, cx| window.click("picker-provider-cinder", cx));
    assert_eq!(
        seen.borrow().last(),
        Some(&(Chosen::Model("cinder".into(), Some("ember".into())), None))
    );
    advance(cx, PANEL / 3);
    let midway = height(cx);
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
    let midway = cx.update(|window, _| window.find("picker-efforts").bounds().size.height);
    advance(cx, PANEL * 2);
    let whole = cx.update(|window, _| window.find("picker-efforts").bounds().size.height);
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
