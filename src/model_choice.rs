//! `ModelChoice`: the one way to choose a model (`docs/reference/gpui.md`, rule 15). One select
//! whose entries are `<provider> · <model>`, grouped by provider, with the effort in a narrow
//! select beside it when the model has efforts. There is never a row named "default": a select
//! opens on the value that applies, selected as if chosen and marked `recommended`, and choosing
//! it back clears the choice. A choice that is not a model comes before the models. "No model" is
//! never an entry: a feature is turned off by its switch.
//!
//! The kit knows no provider: the screen gives the models the application knows ([`ModelEntry`]).

use std::rc::Rc;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};

use crate::field::Field;
use crate::interact::fade;
use crate::motion::PANEL;
use crate::select::{Select, SelectOption};
use crate::space;
use crate::theme::palette;

/// The width of the model's select on a line of a card.
const COMPACT: f32 = 300.;
/// The width of the effort's select: room for `xhigh · recommended`.
const EFFORT: f32 = 170.;
/// What marks the value that applies when nothing is chosen.
const RECOMMENDED: &str = "recommended";
/// The width of the `picked` mark.
const MARK: f32 = 52.;

/// A model the screen offers: its provider (its key and its name) and the model (`None` while the
/// provider's models are not known), with the efforts it takes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelEntry {
    provider: SharedString,
    provider_title: SharedString,
    model: Option<SharedString>,
    name: SharedString,
    efforts: Vec<SharedString>,
    recommended: bool,
    proposes: Option<SharedString>,
}

impl ModelEntry {
    pub fn new(
        provider: impl Into<SharedString>,
        provider_title: impl Into<SharedString>,
        model: Option<SharedString>,
        name: impl Into<SharedString>,
    ) -> Self {
        Self {
            provider: provider.into(),
            provider_title: provider_title.into(),
            model,
            name: name.into(),
            efforts: Vec::new(),
            recommended: false,
            proposes: None,
        }
    }

    /// The model the provider takes when none is named: it is marked `recommended`, and choosing
    /// it reports the provider with no model, so that the choice keeps following the provider.
    pub fn recommended(mut self) -> Self {
        self.recommended = true;
        self
    }

    /// The effort this model runs with when none is chosen: marked `recommended` among its
    /// efforts, and choosing it reports no effort.
    pub fn proposes(mut self, effort: impl Into<SharedString>) -> Self {
        self.proposes = Some(effort.into());
        self
    }

    pub fn efforts(mut self, efforts: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.efforts = efforts.into_iter().map(Into::into).collect();
        self
    }

    /// Whether it stands for `model` of `provider`: a recommended model stands for none named.
    fn is(&self, provider: &str, model: Option<&str>) -> bool {
        self.provider == provider
            && (self.model.as_deref() == model || (self.recommended && model.is_none()))
    }

    /// What choosing it reports.
    fn chosen(&self) -> Chosen {
        let model = self.model.clone().filter(|_| !self.recommended);
        Chosen::Model(self.provider.clone(), model)
    }
}

/// What is chosen: the first option (As in Settings), or, with `follows`, nothing (the first
/// recommended model applies), a choice that is not a model, by its key, or a provider's model
/// (`None` its recommended one).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Chosen {
    First,
    Choice(SharedString),
    Model(SharedString, Option<SharedString>),
}

type OnChoose = Rc<dyn Fn(Chosen, Option<SharedString>, &mut Window, &mut App) + 'static>;

/// The first option: the rule of a screen that inherits Settings' value, and what it gives today.
#[derive(Clone)]
struct First {
    title: SharedString,
    gives: SharedString,
}

#[derive(IntoElement)]
pub struct ModelChoice {
    id: ElementId,
    label: SharedString,
    chosen: Chosen,
    effort: Option<SharedString>,
    first: Option<First>,
    follows: bool,
    choices: Vec<SelectOption>,
    models: Vec<ModelEntry>,
    picked: bool,
    compact: bool,
    name: Option<SharedString>,
    hint: Option<SharedString>,
    cost: Option<SharedString>,
    disabled: bool,
    on_choose: Option<OnChoose>,
}

impl ModelChoice {
    /// The choice `id`: its select `<id>-model` (each option `<id>-model-<key>`), the effort's
    /// `<id>-effort`, the mark `<id>-picked`, the cost line `<id>-cost`.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        chosen: Chosen,
    ) -> Self {
        Self {
            id: ElementId::Name(id.into()),
            label: label.into(),
            chosen,
            effort: None,
            first: None,
            follows: false,
            choices: Vec::new(),
            models: Vec::new(),
            picked: false,
            compact: false,
            name: None,
            hint: None,
            cost: None,
            disabled: false,
            on_choose: None,
        }
    }

    /// Nothing chosen is a rule: the first recommended model applies (the first model when none
    /// is marked), selected and marked, and choosing it back, or its proposed effort back,
    /// reports `Chosen::First`, the choice cleared.
    pub fn follows(mut self) -> Self {
        self.follows = true;
        self
    }

    /// Where a value is inherited: the first option is `As in Settings`, its second line what it
    /// gives.
    pub fn inherited(mut self, gives: impl Into<SharedString>) -> Self {
        self.first = Some(First {
            title: "As in Settings".into(),
            gives: gives.into(),
        });
        self
    }

    /// A choice that is not a model (a summary written by the conversation itself), before the
    /// models.
    pub fn choice(
        mut self,
        key: impl Into<SharedString>,
        title: impl Into<SharedString>,
        line: impl Into<SharedString>,
    ) -> Self {
        self.choices
            .push(SelectOption::new(key, title).line(line.into()));
        self
    }

    pub fn models(mut self, models: impl IntoIterator<Item = ModelEntry>) -> Self {
        self.models = models.into_iter().collect();
        self
    }

    /// The effort chosen with the model, `None` for the one it proposes.
    pub fn effort(mut self, effort: Option<SharedString>) -> Self {
        self.effort = effort;
        self
    }

    /// The application picked the model chosen, and the user has not seen its notice yet.
    pub fn picked(mut self, picked: bool) -> Self {
        self.picked = picked;
        self
    }

    /// A line of a card: the label on the left, one line under it, the selects on the right.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// A compact choice on a line whose own words already name it (a switch's label): it draws
    /// neither its label nor its hint, only its selects, and `name` is the accessible name of
    /// `<id>`. Of no effect on a choice that is not compact.
    pub fn unlabelled(mut self, name: impl Into<SharedString>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// The line under the label of a compact choice, `<id>-hint`.
    pub fn hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// What the model costs, one line under it.
    pub fn cost(mut self, cost: impl Into<SharedString>) -> Self {
        self.cost = Some(cost.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called with what is chosen and its effort, when it changes.
    pub fn on_choose(
        mut self,
        f: impl Fn(Chosen, Option<SharedString>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_choose = Some(Rc::new(f));
        self
    }
}

fn part(id: &ElementId, name: &str) -> SharedString {
    format!("{id}-{name}").into()
}

/// `content` beside its sibling, unfolding to `width` and fading in while `present`, folding
/// away and fading out when not: nothing at all once it is gone.
fn beside(
    id: SharedString,
    present: bool,
    width: f32,
    content: AnyElement,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let t = fade(ElementId::Name(id.clone()), present, PANEL, window, cx);
    if t == 0.0 && !present {
        return div().hidden().into_any_element();
    }
    div()
        .id(ElementId::Name(id))
        .flex_none()
        .overflow_hidden()
        .w(px((width + f32::from(space::SM)) * t))
        .opacity(t)
        .child(
            div()
                .pl(space::SM)
                .w(px(width + f32::from(space::SM)))
                .child(content),
        )
        .test_support()
        .into_any_element()
}

impl RenderOnce for ModelChoice {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let id = self.id.clone();
        let mut models = self.models.clone();
        // A model chosen that is not offered (written by hand, or of a provider not ready) is
        // still shown.
        if let Chosen::Model(provider, model) = &self.chosen
            && !models.iter().any(|m| m.is(provider, model.as_deref()))
        {
            let title = models
                .iter()
                .find(|m| &m.provider == provider)
                .map_or_else(|| provider.clone(), |m| m.provider_title.clone());
            let name = model
                .clone()
                .unwrap_or_else(|| "The provider's model".into());
            models.push(ModelEntry::new(
                provider.clone(),
                title,
                model.clone(),
                name,
            ));
        }
        let mut options = Vec::new();
        let mut targets = Vec::new();
        if let Some(first) = &self.first {
            options.push(SelectOption::new("first", first.title.clone()).line(first.gives.clone()));
            targets.push(Chosen::First);
        }
        for choice in &self.choices {
            targets.push(Chosen::Choice(choice.key()));
            options.push(choice.clone());
        }
        for (at, model) in models.iter().enumerate() {
            options.push(
                SelectOption::new(
                    format!(
                        "{}-{}",
                        model.provider,
                        model.model.as_deref().unwrap_or("recommended")
                    )
                    .replace(['/', ' '], "-")
                        + if at < self.models.len() { "" } else { "-own" },
                    if model.recommended {
                        format!("{} · {} · {RECOMMENDED}", model.provider_title, model.name)
                    } else {
                        format!("{} · {}", model.provider_title, model.name)
                    },
                )
                .group(model.provider_title.clone()),
            );
            targets.push(model.chosen());
        }
        // Where nothing is chosen, the first recommended model applies.
        let applying = models.iter().position(|m| m.recommended).unwrap_or(0);
        let shown = match &self.chosen {
            Chosen::First if self.follows => Some(applying),
            chosen => models.iter().position(|m| match chosen {
                Chosen::Model(provider, model) => m.is(provider, model.as_deref()),
                _ => false,
            }),
        };
        let offset = options.len() - models.len();
        let cleared = self.follows.then_some(offset + applying);
        let at = shown
            .map(|at| offset + at)
            .or_else(|| targets.iter().position(|t| *t == self.chosen))
            .unwrap_or(0);
        let entry = shown.and_then(|at| models.get(at));
        let efforts = entry.map(|m| m.efforts.clone()).unwrap_or_default();
        let proposes = entry.and_then(|m| m.proposes.clone());
        let has_efforts = !efforts.is_empty();
        let on_choose = self.on_choose.clone();
        let model = Select::new(part(&id, "model"), options, at)
            .disabled(self.disabled)
            .on_choose({
                let on_choose = on_choose.clone();
                move |index, window, cx| {
                    if let Some(on_choose) = &on_choose {
                        let chosen = match cleared {
                            Some(cleared) if index == cleared => Chosen::First,
                            _ => targets[index].clone(),
                        };
                        on_choose(chosen, None, window, cx);
                    }
                }
            });
        let model = if self.compact {
            model.width(px(COMPACT))
        } else {
            model
        };
        // The effort that applies is selected when none is chosen, and marked: no row is named
        // "default".
        let mut levels: Vec<SharedString> = efforts.clone();
        if has_efforts
            && let Some(own) = &self.effort
            && !efforts.contains(own)
        {
            levels.push(own.clone());
        }
        let effort_at = self
            .effort
            .as_ref()
            .or(proposes.as_ref())
            .and_then(|level| levels.iter().position(|l| l == level))
            .unwrap_or(0);
        let effort = Select::new(
            part(&id, "effort"),
            levels.iter().map(|level| {
                let said = if proposes.as_ref() == Some(level) {
                    format!("{level} · {RECOMMENDED}")
                } else {
                    level.to_string()
                };
                SelectOption::new(level.clone(), said)
            }),
            effort_at,
        )
        .width(px(EFFORT))
        .disabled(self.disabled)
        .on_choose({
            // The model that applies with the effort it proposes is no choice at all.
            let clears = self.follows && shown == Some(applying);
            let chosen = entry.map_or_else(|| self.chosen.clone(), ModelEntry::chosen);
            move |index, window, cx| {
                if let Some(on_choose) = &on_choose {
                    let level = levels[index].clone();
                    let effort = (proposes.as_ref() != Some(&level)).then_some(level);
                    // An effort needs its provider named; the recommended one clears all.
                    let chosen = match (&effort, clears) {
                        (None, true) => Chosen::First,
                        _ => chosen.clone(),
                    };
                    on_choose(chosen, effort, window, cx);
                }
            }
        });
        let mark = div()
            .id(ElementId::Name(part(&id, "picked-mark")))
            .aria_label("picked")
            .px(space::SM)
            .py(space::XS)
            .rounded(px(6.))
            .bg(p.card2)
            .text_size(px(11.5))
            .text_color(p.mute)
            .child("picked")
            .test_support();
        let row = div()
            .flex()
            .items_center()
            .min_w_0()
            .when(!self.compact, |d| d.w_full())
            .child(
                div()
                    .min_w_0()
                    .when(!self.compact, |d| d.flex_1())
                    .child(model),
            )
            .child(beside(
                part(&id, "effort-block"),
                has_efforts,
                EFFORT,
                effort.into_any_element(),
                window,
                cx,
            ))
            .child(beside(
                part(&id, "picked"),
                self.picked,
                MARK,
                mark.into_any_element(),
                window,
                cx,
            ));
        let cost = self.cost.clone().map(|cost| {
            div()
                .id(ElementId::Name(part(&id, "cost")))
                .aria_label(cost.clone())
                .pt(space::SM)
                .text_size(px(12.))
                .text_color(p.faint)
                .child(cost)
                .test_support()
        });
        if self.compact
            && let Some(name) = self.name.clone()
        {
            return div()
                .id(id)
                .aria_label(name)
                .flex()
                .flex_col()
                .child(div().flex().items_center().child(row))
                .children(cost)
                .test_support()
                .into_any_element();
        }
        if self.compact {
            let words = div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .id(ElementId::Name(part(&id, "label")))
                        .aria_label(self.label.clone())
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.text)
                        .child(self.label.clone())
                        .test_support(),
                )
                .children(self.hint.clone().map(|hint| {
                    div()
                        .id(ElementId::Name(part(&id, "hint")))
                        .aria_label(hint.clone())
                        .text_size(px(12.))
                        .text_color(p.faint)
                        .child(hint)
                        .test_support()
                }));
            return div()
                .id(id)
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(space::MD)
                        .child(words)
                        .child(row),
                )
                .children(cost)
                .test_support()
                .into_any_element();
        }
        div()
            .id(id.clone())
            .flex()
            .flex_col()
            .child(Field::new(part(&id, "field"), self.label.clone(), row))
            .children(cost)
            .test_support()
            .into_any_element()
    }
}
