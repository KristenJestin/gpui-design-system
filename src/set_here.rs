//! `SetHere`: the mark of a value set at one level over the one it would inherit from another
//! (a language chosen for one screen over the one set for all): a dot and "set here" under the
//! control, and a way back to the inherited value. The mark unfolds and fades in when the value is
//! set and folds away and fades out when it is not. The words are the application's.

use std::rc::Rc;

use gpui_kit::{
    AnyElement, App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};

use crate::color::mix;
use crate::interact::hoverable;
use crate::reveal::presence;
use crate::space;
use crate::theme::palette;

type OnReset = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct SetHere {
    id: ElementId,
    control: AnyElement,
    set: bool,
    set_label: SharedString,
    reset_label: SharedString,
    on_reset: Option<OnReset>,
}

impl SetHere {
    /// `control` (a field, a select, a model choice) with the mark under it. `set_label` is what
    /// the mark says (`set here`) and `reset_label` what the way back says (`Use Settings'
    /// value`). The mark is `<id>-set`, the way back `<id>-reset`.
    pub fn new(
        id: impl Into<ElementId>,
        control: impl IntoElement,
        set_label: impl Into<SharedString>,
        reset_label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            control: control.into_any_element(),
            set: false,
            set_label: set_label.into(),
            reset_label: reset_label.into(),
            on_reset: None,
        }
    }

    /// Whether the value is set here: the mark is drawn while it is.
    pub fn set(mut self, set: bool) -> Self {
        self.set = set;
        self
    }

    /// Called when the way back is clicked; the screen clears its value.
    pub fn on_reset(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_reset = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for SetHere {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let id_text = match &self.id {
            ElementId::Name(name) => name.to_string(),
            other => format!("{other:?}"),
        };
        let part = |tail: &str| ElementId::Name(format!("{id_text}-{tail}").into());
        let back = div()
            .id(part("reset"))
            .aria_label(self.reset_label.clone())
            .cursor_pointer()
            .text_size(px(12.))
            .child(self.reset_label);
        let back = match self.on_reset {
            Some(on_reset) => back.on_click(move |_: &ClickEvent, window, cx| on_reset(window, cx)),
            None => back,
        };
        let line = div()
            .flex()
            .items_center()
            .gap(space::SM)
            .child(
                div()
                    .id(part("set"))
                    .aria_label(self.set_label.clone())
                    .flex()
                    .items_center()
                    .gap(space::XS)
                    .text_size(px(12.))
                    .text_color(p.acc)
                    .child(div().size(px(6.)).rounded_full().bg(p.acc))
                    .child(self.set_label)
                    .test_support(),
            )
            .child(hoverable(back, move |link, hover| {
                link.text_color(mix(p.faint, p.text, hover))
            }));
        div()
            .flex()
            .flex_col()
            .gap(space::SM)
            .child(self.control)
            .child(presence(part("block"), self.set, line).gap(space::SM))
    }
}
