//! `QrCode`: a QR code for a text, drawn in the theme's colours.
//!
//! Encoding the text into modules is the `qrcodegen` crate's; the kit draws them. Scanners expect
//! dark modules on a light ground, so the code sits on a plate that is light in both themes (the
//! theme's field colour in a light one, its text colour in a dark one) and its modules are the
//! darkest ink the theme has. The plate holds the quiet zone: 4 modules of it on every side.
//!
//! The error correction is Medium (the encoder raises it when that costs no size): a code on a
//! screen is not scratched or smudged, so the most it must survive is glare and a moiré, and
//! Medium keeps a text of 180 characters (a link and an upper-case base32 code) at 53 modules,
//! which are 4 px each in the default size. A text over 2331 bytes does not fit at Medium and draws nothing.
//!
//! Its parts, for a test: the block `<id>` (it folds in and out), the code `<id>-code` (labelled),
//! the plate `<id>-plate` and the grid of modules `<id>-modules`.

use gpui_kit::{
    App, Bounds, ElementId, InteractiveElement, IntoElement, ParentElement, Pixels, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, canvas, div, fill,
    point, px, size,
};
use qrcodegen::QrCodeEcc;

use crate::color::readable;
use crate::ids::part;
use crate::reveal::presence;
use crate::seen;
use crate::space;
use crate::theme::{Palette, palette};

/// The modules of quiet zone a scanner needs on every side of the code.
const QUIET_ZONE: usize = 4;
/// The least contrast between the modules and the plate.
const CONTRAST: f32 = 7.0;

/// A QR code for a text, at most `size` px a side (the plate with its quiet zone: the module is
/// the whole number of pixels that fits, so the edges stay sharp), named for assistive technology by `label` (what it holds, never the text
/// itself). It unfolds and fades in when it is `shown` and folds away and fades out when it is
/// not, as any block of the kit does.
#[derive(IntoElement)]
pub struct QrCode {
    id: ElementId,
    text: SharedString,
    label: SharedString,
    size: Pixels,
    shown: bool,
}

impl QrCode {
    /// The code `id` of `text`, labelled `label` ("Invitation code"). It is shown, 288 px
    /// ([`space::XXXL`] × 6) a side.
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

    /// The side of the code, a multiple of a step of the spacing scale (`space::XXXL * 4.`).
    pub fn size(mut self, size: Pixels) -> Self {
        self.size = size;
        self
    }

    /// Whether the code is there. Build it on every frame, also when it is not shown: it is its
    /// own state, as [`presence`] is.
    pub fn shown(mut self, shown: bool) -> Self {
        self.shown = shown;
        self
    }
}

/// The plate and the modules: dark on light whatever the theme is.
fn colours(p: &Palette) -> (gpui_kit::Hsla, gpui_kit::Hsla) {
    let (plate, ink) = if p.bg.l > 0.5 {
        (p.field, p.text)
    } else {
        (p.text, p.body)
    };
    (plate, readable(ink, plate, CONTRAST))
}

impl RenderOnce for QrCode {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let code = div()
            .id(part(&self.id, "code"))
            .aria_label(self.label.clone())
            .flex()
            .items_center()
            .justify_center();
        let content = match qrcodegen::QrCode::encode_text(&self.text, QrCodeEcc::Medium) {
            Ok(encoded) => {
                let (plate, ink) = colours(&palette(cx));
                let side = encoded.size() as usize;
                let module = (f32::from(self.size) / (side + 2 * QUIET_ZONE) as f32)
                    .floor()
                    .max(1.0);
                // A row's dark modules side by side are one quad: fewer to paint, and no seam
                // between two neighbours.
                let mut runs: Vec<(usize, usize, usize)> = Vec::new();
                let mut dark = 0;
                for y in 0..side {
                    let mut start = None;
                    for x in 0..=side {
                        let on = x < side && encoded.get_module(x as i32, y as i32);
                        dark += usize::from(on);
                        match (on, start) {
                            (true, None) => start = Some(x),
                            (false, Some(from)) => {
                                runs.push((from, y, x - from));
                                start = None;
                            }
                            _ => {}
                        }
                    }
                }
                let modules_id = part(&self.id, "modules");
                seen::note(&modules_id, dark as f32);
                seen::note_colour(&modules_id, ink);
                seen::note_colour(&part(&self.id, "plate"), plate);
                let grid = px(module * side as f32);
                let edge = px(module * (side + 2 * QUIET_ZONE) as f32);
                code.child(
                    div()
                        .id(part(&self.id, "plate"))
                        .flex_none()
                        .w(edge)
                        .h(edge)
                        .bg(plate)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .id(modules_id)
                                .flex_none()
                                .w(grid)
                                .h(grid)
                                .child(
                                    canvas(
                                        |_, _, _| (),
                                        move |bounds: Bounds<Pixels>, (), window, _| {
                                            for (x, y, len) in runs {
                                                let origin = bounds.origin
                                                    + point(
                                                        px(module * x as f32),
                                                        px(module * y as f32),
                                                    );
                                                window.paint_quad(fill(
                                                    Bounds::new(
                                                        origin,
                                                        size(px(module * len as f32), px(module)),
                                                    ),
                                                    ink,
                                                ));
                                            }
                                        },
                                    )
                                    .size_full(),
                                )
                                .test_support(),
                        )
                        .test_support(),
                )
            }
            Err(_) => code,
        };
        presence(self.id, self.shown, content.test_support())
    }
}
