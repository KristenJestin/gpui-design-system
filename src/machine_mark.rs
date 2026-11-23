//! [`MachineMark`]: a machine's own icon in its own colour, with a small badge on its corner that
//! says its state by shape.

use gpui_kit::{
    App, ElementId, Hsla, InteractiveElement, IntoElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Window, div,
};

use crate::icons::Icon;

/// What a machine is doing, said by the shape of the badge on its mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MachineState {
    /// The machine this is running on: a house.
    Here,
    /// Reachable: a filled dot.
    Online,
    /// Switched off or gone: a hollow ring, the icon greyed.
    Offline,
    /// Reachable but out of date: a dot with an up arrow.
    Update,
    /// Not reachable, or not tried yet: a dashed ring.
    Unknown,
}

/// The three sizes of a mark, by the side of its icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkSize {
    /// 16 px: a heading.
    Large,
    /// 14 px: a mark in a row.
    Medium,
    /// 12 px: a mark in a line of small text.
    Small,
}

#[derive(IntoElement)]
pub struct MachineMark {
    id: ElementId,
    label: SharedString,
}

impl MachineMark {
    pub fn new(
        id: impl Into<ElementId>,
        _icon: Icon,
        _colour: Hsla,
        _state: MachineState,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }

    pub fn size(self, _size: MarkSize) -> Self {
        self
    }

    pub fn on(self, _surface: Hsla) -> Self {
        self
    }
}

impl RenderOnce for MachineMark {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().id(self.id).aria_label(self.label)
    }
}
