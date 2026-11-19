//! `QrCode`: a QR code for a text, on a plate that scans in every theme.
//!
//! Encoding the text into modules is the `qrcodegen` crate's; the kit draws them. Scanners expect
//! dark modules on a light ground, so the plate is a near-white and the modules a near-black in
//! every theme, dark ones included: the polarity of a code must not follow the palette, which
//! blends through mid-greys while a theme changes. The plate holds the quiet zone: 4 modules of it
//! on every side.
//!
//! The error correction is Medium (the encoder raises it when that costs no size): a code on a
//! screen is not scratched or smudged, so the most it must survive is glare and a moiré, and
//! Medium keeps a text of 180 characters (a link and an upper-case base32 code) at 53 modules,
//! which are 4 px each in the default size. A text over 2331 bytes does not fit at Medium: it
//! draws nothing at all, no plate, no label, and nothing unfolds.
//!
//! Its parts, for a test: the block `<id>` (it folds in and out), the code `<id>-code` (labelled),
//! the plate `<id>-plate` and the grid of modules `<id>-modules`.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use gpui_kit::{
    AnyElement, App, Bounds, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement,
    Pixels, RenderOnce, SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    canvas, div, fill, point, px, size,
};
use qrcodegen::QrCodeEcc;

use crate::color::hex;
use crate::ids::part;
use crate::reveal::presence;
use crate::seen;
use crate::space;

/// The modules of quiet zone a scanner needs on every side of the code.
const QUIET_ZONE: usize = 4;

/// The plate: a near-white, whatever the theme.
fn plate() -> Hsla {
    hex(0xfafafa)
}

/// The modules: a near-black, whatever the theme (a contrast of about 18 with the plate).
fn ink() -> Hsla {
    hex(0x111111)
}

/// The quads a grid painted: their bounds from the grid's origin, and their colour.
type Quads = Vec<(Bounds<Pixels>, Hsla)>;

thread_local! {
    static PAINTED: RefCell<HashMap<ElementId, Quads>> = RefCell::new(HashMap::new());
    static ENCODES: Cell<usize> = const { Cell::new(0) };
}

/// What the grid `id` last painted, for the kit's own tests: each quad's bounds from the grid's
/// origin, and its colour.
#[doc(hidden)]
pub fn painted(id: impl Into<ElementId>) -> Quads {
    PAINTED.with(|painted| {
        painted
            .borrow()
            .get(&id.into())
            .cloned()
            .unwrap_or_default()
    })
}

/// How many times a text was encoded on this thread, for the kit's own tests.
#[doc(hidden)]
pub fn encodes() -> usize {
    ENCODES.with(Cell::get)
}

/// A text encoded: the modules a side, and the dark ones as runs `(x, y, length)`. A row's dark
/// modules side by side are one run: fewer quads to paint, and no seam between two neighbours.
struct Grid {
    side: usize,
    runs: Vec<(usize, usize, usize)>,
}

/// What an element remembers between frames: the text it encoded last (so a frame does not
/// encode it again) and what came of it, `None` when the text does not fit.
struct Encoded {
    text: Option<SharedString>,
    grid: Option<Rc<Grid>>,
}

fn encode(text: &str) -> Option<Grid> {
    ENCODES.with(|count| count.set(count.get() + 1));
    let code = qrcodegen::QrCode::encode_text(text, QrCodeEcc::Medium).ok()?;
    let side = code.size() as usize;
    let mut runs = Vec::new();
    for y in 0..side {
        let mut start = None;
        for x in 0..=side {
            let dark = x < side && code.get_module(x as i32, y as i32);
            match (dark, start) {
                (true, None) => start = Some(x),
                (false, Some(from)) => {
                    runs.push((from, y, x - from));
                    start = None;
                }
                _ => {}
            }
        }
    }
    Some(Grid { side, runs })
}

/// A QR code for a text, named for assistive technology by `label` (what it holds, never the text
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

    /// The most the code takes a side, plate and quiet zone included: the module is the whole
    /// number of pixels that fits, so the edges stay sharp and the plate can be a little smaller.
    /// The module is at least 1 px, so under (modules + 8) px the plate is larger than `size`.
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

impl RenderOnce for QrCode {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(part(&self.id, "encoded"), cx, |_, _| Encoded {
            text: None,
            grid: None,
        });
        if state.read(cx).text.as_ref() != Some(&self.text) {
            let grid = encode(&self.text).map(Rc::new);
            state.update(cx, |encoded, _| {
                encoded.text = Some(self.text.clone());
                encoded.grid = grid;
            });
        }
        let Some(grid) = state.read(cx).grid.clone() else {
            return div().hidden().into_any_element();
        };
        let side = grid.side;
        let module = (f32::from(self.size) / (side + 2 * QUIET_ZONE) as f32)
            .floor()
            .max(1.0);
        let (plate, ink) = (plate(), ink());
        let modules_id = part(&self.id, "modules");
        let painting = modules_id.clone();
        seen::note_colour(&modules_id, ink);
        seen::note_colour(&part(&self.id, "plate"), plate);
        let grid_size = px(module * side as f32);
        let edge = px(module * (side + 2 * QUIET_ZONE) as f32);
        let modules = div()
            .id(modules_id)
            .flex_none()
            .w(grid_size)
            .h(grid_size)
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds: Bounds<Pixels>, (), window, _| {
                        let mut quads = Vec::new();
                        for &(x, y, len) in &grid.runs {
                            let at = point(px(module * x as f32), px(module * y as f32));
                            let quad = Bounds::new(
                                bounds.origin + at,
                                size(px(module * len as f32), px(module)),
                            );
                            window.paint_quad(fill(quad, ink));
                            quads.push((Bounds::new(at, quad.size), ink));
                        }
                        PAINTED.with(|painted| painted.borrow_mut().insert(painting, quads));
                    },
                )
                .size_full(),
            )
            .test_support();
        let plate = div()
            .id(part(&self.id, "plate"))
            .flex_none()
            .w(edge)
            .h(edge)
            .bg(plate)
            .flex()
            .items_center()
            .justify_center()
            .child(modules)
            .test_support();
        let code: AnyElement = div()
            .id(part(&self.id, "code"))
            .aria_label(self.label)
            .flex()
            .items_center()
            .justify_center()
            .child(plate)
            .test_support()
            .into_any_element();
        presence(self.id, self.shown, code).into_any_element()
    }
}
