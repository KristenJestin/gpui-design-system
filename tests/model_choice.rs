//! `ModelChoice`, the one way to choose a model (`docs/reference/gpui.md`, rule 15), on GPUI's
//! test platform: it opens on the recommended model selected, never a row named "default", no
//! "none" entry, the effort beside a model that has efforts only.

mod common;

use std::cell::RefCell;
use std::rc::Rc;

use common::{advance, init, pending_frames};
use gpui_design_system::motion::{ENTER, PANEL};
use gpui_design_system::{Chosen, ModelChoice, ModelEntry};
use gpui_kit::test::{TestQueryExt as _, TestWindowExt};
use gpui_kit::{
    Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled, TestAppContext,
    VisualTestContext, Window, div, px, size,
};

/// What a choice reported: what was chosen, and its effort.
type Seen = Rc<RefCell<Vec<(Chosen, Option<SharedString>)>>>;

/// A window holding one choice, which keeps what was chosen.
struct Host {
    chosen: Chosen,
    effort: Option<SharedString>,
    picked: bool,
    models: Vec<ModelEntry>,
    seen: Seen,
}

fn models() -> Vec<ModelEntry> {
    vec![
        ModelEntry::new("claude", "Claude", Some("opus".into()), "Opus")
            .efforts(["low", "high"])
            .recommended()
            .proposes("high"),
        ModelEntry::new("claude", "Claude", Some("haiku".into()), "Haiku"),
        ModelEntry::new("codex", "Codex", Some("gpt-5".into()), "GPT-5")
            .efforts(["low", "medium"])
            .proposes("medium"),
    ]
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (this, seen) = (cx.entity().downgrade(), self.seen.clone());
        let choice = ModelChoice::new("judge", "Model", self.chosen.clone())
            .follows()
            .models(self.models.clone())
            .effort(self.effort.clone())
            .picked(self.picked)
            .cost("Each decision is a turn of this model, on your own quota.")
            .on_choose(move |chosen, effort, _, cx| {
                seen.borrow_mut().push((chosen.clone(), effort.clone()));
                this.update(cx, |this, cx| {
                    this.chosen = chosen;
                    this.effort = effort;
                    cx.notify();
                })
                .ok();
            });
        div().size_full().p(px(16.)).child(choice)
    }
}

fn host(cx: &mut TestAppContext, chosen: Chosen) -> (Entity<Host>, &mut VisualTestContext, Seen) {
    host_with(cx, chosen, models())
}

fn host_with(
    cx: &mut TestAppContext,
    chosen: Chosen,
    models: Vec<ModelEntry>,
) -> (Entity<Host>, &mut VisualTestContext, Seen) {
    init(cx);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let given = seen.clone();
    let (view, cx) = cx.add_window_view(move |_, _| Host {
        chosen,
        effort: None,
        picked: false,
        models,
        seen: given,
    });
    cx.simulate_resize(size(px(900.), px(700.)));
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx, seen)
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

fn open(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.click("judge-model", cx));
    advance(cx, ENTER * 2);
}

#[gpui_kit::test]
fn a_model_choice_opens_on_the_recommended_model_selected(cx: &mut TestAppContext) {
    let (_, cx, seen) = host(cx, Chosen::First);
    assert_eq!(
        label("judge-model", cx).as_deref(),
        Some("Claude · Opus · recommended"),
        "nothing chosen: the recommended model is the one selected, marked"
    );
    assert_eq!(
        label("judge-cost", cx).as_deref(),
        Some("Each decision is a turn of this model, on your own quota.")
    );
    open(cx);
    // The models, `<provider> · <model>`, grouped by provider; no row is the rule.
    assert_eq!(
        label("judge-model-claude-opus", cx).as_deref(),
        Some("Claude · Opus · recommended")
    );
    assert_eq!(
        label("judge-model-claude-haiku", cx).as_deref(),
        Some("Claude · Haiku")
    );
    assert!(label("judge-model-first", cx).is_none());
    assert_eq!(label("judge-model-group-1", cx).as_deref(), Some("Claude"));
    assert_eq!(label("judge-model-group-2", cx).as_deref(), Some("Codex"));
    assert!(label("judge-model-group-3", cx).is_none());
    cx.update(|window, cx| window.click("judge-model-codex-gpt-5", cx));
    assert_eq!(
        seen.borrow().last(),
        Some(&(Chosen::Model("codex".into(), Some("gpt-5".into())), None))
    );
    advance(cx, PANEL * 2);
    assert_eq!(label("judge-model", cx).as_deref(), Some("Codex · GPT-5"));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn choosing_the_recommended_model_back_clears_the_choice(cx: &mut TestAppContext) {
    let (_, cx, seen) = host(cx, Chosen::Model("claude".into(), Some("haiku".into())));
    open(cx);
    cx.update(|window, cx| window.click("judge-model-claude-opus", cx));
    assert_eq!(
        seen.borrow().last(),
        Some(&(Chosen::First, None)),
        "the recommended one is no choice: it keeps following the provider"
    );
    advance(cx, PANEL * 2);
    assert_eq!(
        label("judge-model", cx).as_deref(),
        Some("Claude · Opus · recommended")
    );
}

#[gpui_kit::test]
fn a_model_choice_has_no_none_entry(cx: &mut TestAppContext) {
    let (_, cx, _) = host(cx, Chosen::Model("claude".into(), Some("haiku".into())));
    open(cx);
    let options: Vec<String> = cx.update(|window, _| {
        ["claude-opus", "claude-haiku", "codex-gpt-5"]
            .iter()
            .map(|key| {
                window
                    .find(format!("judge-model-{key}"))
                    .label()
                    .unwrap()
                    .to_owned()
            })
            .collect()
    });
    for option in &options {
        assert!(
            !option.to_lowercase().contains("none") && !option.starts_with("No "),
            "{option}"
        );
    }
    cx.update(|window, _| {
        assert!(window.try_find("judge-model-none").is_none());
        assert!(window.try_find_by_label("None").is_none());
    });
}

#[gpui_kit::test]
fn the_effort_unfolds_only_for_a_model_with_efforts(cx: &mut TestAppContext) {
    let (view, cx, seen) = host(cx, Chosen::Model("claude".into(), Some("haiku".into())));
    assert!(!shown("judge-effort", cx), "Haiku has no effort");
    open(cx);
    cx.update(|window, cx| window.click("judge-model-claude-opus", cx));
    // It unfolds beside the model: narrower than its full width midway, whole at the end.
    advance(cx, PANEL / 3);
    let midway = cx.update(|window, _| window.find("judge-effort-block").bounds().size.width);
    advance(cx, PANEL);
    let whole = cx.update(|window, _| window.find("judge-effort-block").bounds().size.width);
    assert!(midway < whole, "{midway:?} < {whole:?}");
    assert!(shown("judge-effort", cx));
    assert_eq!(
        label("judge-effort", cx).as_deref(),
        Some("high · recommended"),
        "the effort that applies is selected, marked, never a row named default"
    );
    cx.update(|window, cx| window.click("judge-effort", cx));
    advance(cx, ENTER * 2);
    cx.update(|window, cx| window.click("judge-effort-low", cx));
    assert_eq!(
        seen.borrow().last(),
        Some(&(Chosen::Model("claude".into(), None), Some("low".into())))
    );
    advance(cx, PANEL * 2);
    // Back on the recommended effort: the choice clears.
    cx.update(|window, cx| window.click("judge-effort", cx));
    advance(cx, ENTER * 2);
    cx.update(|window, cx| window.click("judge-effort-high", cx));
    assert_eq!(seen.borrow().last(), Some(&(Chosen::First, None)));
    advance(cx, PANEL * 2);
    // A model without efforts: it folds away.
    view.update(cx, |host, cx| {
        host.chosen = Chosen::Model("claude".into(), Some("haiku".into()));
        host.effort = None;
        cx.notify();
    });
    advance(cx, PANEL / 3);
    assert!(shown("judge-effort-block", cx), "it folds, not vanishes");
    advance(cx, PANEL);
    assert!(!shown("judge-effort-block", cx));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn no_model_choice_offers_a_default_effort_row(cx: &mut TestAppContext) {
    let (view, cx, _) = host(cx, Chosen::Model("claude".into(), Some("opus".into())));
    cx.update(|window, cx| window.click("judge-effort", cx));
    advance(cx, ENTER * 2);
    // The efforts, the recommended one marked; no row is "default" nor stands for an effort
    // unnamed.
    for (key, said) in [("high", "high · recommended"), ("low", "low")] {
        assert_eq!(
            label(&format!("judge-effort-{key}"), cx).as_deref(),
            Some(said)
        );
    }
    cx.update(|window, _| {
        assert!(window.try_find("judge-effort-default").is_none());
        assert!(window.try_find_by_label("Default").is_none());
    });
    cx.update(|window, cx| window.click("judge-effort-low", cx));
    advance(cx, PANEL * 2);
    // The model row neither: open the model's select on every model.
    open(cx);
    for key in ["claude-opus", "claude-haiku", "codex-gpt-5"] {
        let said = label(&format!("judge-model-{key}"), cx).unwrap();
        assert!(!said.to_lowercase().contains("default"), "{said}");
    }
    cx.update(|window, cx| window.click("judge-model-codex-gpt-5", cx));
    advance(cx, PANEL * 2);
    view.update(cx, |_, cx| cx.notify());
    assert_eq!(
        label("judge-effort", cx).as_deref(),
        Some("medium · recommended")
    );
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn the_picked_mark_fades_away_once_seen(cx: &mut TestAppContext) {
    let (view, cx, _) = host(cx, Chosen::Model("claude".into(), Some("haiku".into())));
    view.update(cx, |host, cx| {
        host.picked = true;
        cx.notify();
    });
    advance(cx, PANEL * 2);
    assert_eq!(label("judge-picked-mark", cx).as_deref(), Some("picked"));
    view.update(cx, |host, cx| {
        host.picked = false;
        cx.notify();
    });
    advance(cx, PANEL / 3);
    assert!(shown("judge-picked", cx), "it fades, not vanishes");
    advance(cx, PANEL);
    assert!(!shown("judge-picked", cx));
    assert_eq!(pending_frames(cx), 0);
}

/// A compact choice on a line a switch already names: no words of its own, still named.
struct Bare;

impl Render for Bare {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(700.)).p(px(16.)).flex().child(
            ModelChoice::new("judge", "Model", Chosen::First)
                .inherited("Opus for Claude")
                .models(models())
                .hint("Which model")
                .compact()
                .unlabelled("Bugs model"),
        )
    }
}

#[gpui_kit::test]
fn a_compact_model_choice_without_label_keeps_its_accessible_name(cx: &mut TestAppContext) {
    init(cx);
    let (_, cx) = cx.add_window_view(|_, _| Bare);
    cx.update(|window, cx| window.render_frame(cx));
    cx.update(|window, _| {
        assert_eq!(window.find("judge").label(), Some("Bugs model"));
        assert!(window.try_find("judge-label").is_none(), "no label drawn");
        assert!(window.try_find("judge-hint").is_none(), "nor its hint");
        // Its select is at the left edge of the line: no empty element grew before it.
        assert_eq!(
            window.find("judge-model").bounds().origin.x,
            window.find("judge").bounds().origin.x
        );
    });
}

/// Chooses `level` in the effort's select.
fn choose_effort(level: &str, cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.click("judge-effort", cx));
    advance(cx, ENTER * 2);
    let key = format!("judge-effort-{level}");
    cx.update(|window, cx| window.click(key, cx));
    advance(cx, PANEL * 2);
}

#[gpui_kit::test]
fn choosing_the_recommended_effort_back_clears_the_choice(cx: &mut TestAppContext) {
    let (_, cx, seen) = host(cx, Chosen::First);
    choose_effort("low", cx);
    assert_eq!(
        seen.borrow().last(),
        Some(&(Chosen::Model("claude".into(), None), Some("low".into())))
    );
    // The model that applies, with the effort it proposes: nothing is chosen any more.
    choose_effort("high", cx);
    assert_eq!(seen.borrow().last(), Some(&(Chosen::First, None)));
    assert_eq!(
        label("judge-model", cx).as_deref(),
        Some("Claude · Opus · recommended")
    );
    assert_eq!(
        label("judge-effort", cx).as_deref(),
        Some("high · recommended")
    );
}

#[gpui_kit::test]
fn the_model_that_applies_named_with_its_effort_back_clears(cx: &mut TestAppContext) {
    let (_, cx, seen) = host(cx, Chosen::Model("claude".into(), Some("opus".into())));
    choose_effort("low", cx);
    choose_effort("high", cx);
    assert_eq!(seen.borrow().last(), Some(&(Chosen::First, None)));
}

#[gpui_kit::test]
fn the_recommended_effort_of_another_model_keeps_its_model(cx: &mut TestAppContext) {
    let (_, cx, seen) = host(cx, Chosen::Model("codex".into(), Some("gpt-5".into())));
    choose_effort("low", cx);
    choose_effort("medium", cx);
    assert_eq!(
        seen.borrow().last(),
        Some(&(Chosen::Model("codex".into(), Some("gpt-5".into())), None)),
        "a model that does not apply stays chosen with the effort it proposes"
    );
}

#[gpui_kit::test]
fn with_no_recommended_entry_the_first_model_applies(cx: &mut TestAppContext) {
    let models = vec![
        ModelEntry::new("claude", "Claude", Some("opus".into()), "Opus")
            .efforts(["low", "high"])
            .proposes("high"),
        ModelEntry::new("codex", "Codex", Some("gpt-5".into()), "GPT-5"),
    ];
    let (_, cx, seen) = host_with(cx, Chosen::First, models);
    assert_eq!(
        label("judge-model", cx).as_deref(),
        Some("Claude · Opus"),
        "nothing is marked when the screen marks nothing"
    );
    choose_effort("low", cx);
    assert_eq!(
        seen.borrow().last(),
        Some(&(
            Chosen::Model("claude".into(), Some("opus".into())),
            Some("low".into())
        ))
    );
    choose_effort("high", cx);
    assert_eq!(seen.borrow().last(), Some(&(Chosen::First, None)));
}

#[gpui_kit::test]
fn the_recommended_entry_given_first_is_the_one_selected(cx: &mut TestAppContext) {
    // The provider that applies, its models not known yet, before another provider's
    // recommended model: the select opens on it, never on the other provider's.
    let models = vec![
        ModelEntry::new("claude", "Claude", None, "Models not known yet").recommended(),
        ModelEntry::new("codex", "Codex", Some("gpt-5".into()), "GPT-5").recommended(),
    ];
    let (_, cx, seen) = host_with(cx, Chosen::First, models);
    assert_eq!(
        label("judge-model", cx).as_deref(),
        Some("Claude · Models not known yet · recommended")
    );
    open(cx);
    cx.update(|window, cx| window.click("judge-model-codex-gpt-5", cx));
    assert_eq!(
        seen.borrow().last(),
        Some(&(Chosen::Model("codex".into(), None), None))
    );
}
