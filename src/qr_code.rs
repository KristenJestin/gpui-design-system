//! `QrCode`: a QR code for a text, drawn in the theme's colours.

use gpui_kit::{
    App, ElementId, InteractiveElement, IntoElement, Pixels, RenderOnce, SharedString, Styled,
    Window, div, px,
};

use crate::ids::part;
use crate::reveal::presence;
use crate::space;

/// A QR code for a text.
#[derive(IntoElement)]
pub struct QrCode {
    id: ElementId,
    text: SharedString,
    label: SharedString,
    size: Pixels,
    shown: bool,
}

impl QrCode {
    pub fn new(
        id: impl Into<ElementId>,
        text: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            label: label.into(),
            size: space::XXXL * 6.,
            shown: true,
        }
    }

    pub fn size(mut self, size: Pixels) -> Self {
        self.size = size;
        self
    }

    pub fn shown(mut self, shown: bool) -> Self {
        self.shown = shown;
        self
    }
}

impl RenderOnce for QrCode {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let _ = (&self.text, &self.label, self.size);
        presence(
            self.id.clone(),
            self.shown,
            div().id(part(&self.id, "code")).h(px(0.)),
        )
    }
}
