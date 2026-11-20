//! `RemovableChip` and `ChipPicker`.

use std::rc::Rc;

use gpui_kit::base::input::InputState;
use gpui_kit::{
    App, ElementId, Entity, InteractiveElement, IntoElement, RenderOnce, SharedString, Window, div,
};

type OnRemove = Rc<dyn Fn(&mut Window, &mut App) + 'static>;
type OnChange = Rc<dyn Fn(Vec<SharedString>, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct RemovableChip {
    id: ElementId,
}

impl RemovableChip {
    pub fn new(id: impl Into<ElementId>, _label: impl Into<SharedString>) -> Self {
        Self { id: id.into() }
    }
    pub fn detail(self, _detail: impl Into<SharedString>) -> Self {
        self
    }
    pub fn present(self, _present: bool) -> Self {
        self
    }
    pub fn at_rest(self) -> Self {
        self
    }
    pub fn on_remove(self, _f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self
    }
}

impl RenderOnce for RemovableChip {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().id(self.id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChipOption {
    key: SharedString,
}

impl ChipOption {
    pub fn new(key: impl Into<SharedString>, _label: impl Into<SharedString>) -> Self {
        Self { key: key.into() }
    }
    pub fn detail(self, _detail: impl Into<SharedString>) -> Self {
        self
    }
}

#[derive(IntoElement)]
pub struct ChipPicker {
    id: ElementId,
}

impl ChipPicker {
    pub fn new(
        id: impl Into<ElementId>,
        _label: impl Into<SharedString>,
        _input: &Entity<InputState>,
        _options: impl IntoIterator<Item = ChipOption>,
        _chosen: impl IntoIterator<Item = SharedString>,
    ) -> Self {
        Self { id: id.into() }
    }
    pub fn placeholder(self, _placeholder: impl Into<SharedString>) -> Self {
        self
    }
    pub fn on_change(
        self,
        _f: impl Fn(Vec<SharedString>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self
    }
}

impl RenderOnce for ChipPicker {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().id(self.id)
    }
}

#[allow(dead_code)]
fn _unused(_: OnRemove, _: OnChange) {}
