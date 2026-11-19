//! `QrCode`: a QR code drawn in the theme's colours, on GPUI's test platform.

mod common;

use std::time::Duration;

use common::{advance, init, init_as, pending_frames};
use gpui_design_system::appearance::{self, Appearance, Source};
use gpui_design_system::color::{contrast, hex};
use gpui_design_system::motion::{COVER, PANEL};
use gpui_design_system::theme::{Shade, palette, target_palette};
use gpui_design_system::{Identity, Palette, QrCode, Theme, qr_code, seen};
use gpui_kit::test::{TestQueryExt as _, TestWindowExt};
use gpui_kit::{
    Context, Entity, InteractiveElement as _, IntoElement, ParentElement, Render, SharedString,
    Styled, TestAppContext, TestSupportExt as _, VisualTestContext, Window, div, px, size,
};
use qrcodegen::QrCodeEcc;

/// An invented text of the size a pairing payload has: a link and a base32 code, 180 characters.
const INVITE: &str = "https://hub.example.test/join?invite=7GQ2M9XK4TD0VBHW5ZE8RJNC3FY61PAS7GQ2M9XK4TD0VBHW5ZE8RJNC3FY61PAS7GQ2M9XK4TD0VBHW5ZE8RJNC3FY61PAS7GQ2M9XK4TD0VBHW5ZE8RJNC3FY61PAS7GQ2M9XK4TD0VBH";
const LABEL: &str = "Invitation code";

/// A dark theme tinted the way a desktop's can be: a bluish foreground on a bluish ground.
fn tinted(shade: Shade) -> Palette {
    let mut palette = Palette::own(shade);
    palette.bg = hex(0x1a1b2e);
    palette.body = hex(0x12131f);
    palette.field = hex(0x242640);
    palette.text = hex(0xa9b8f5);
    palette
}

/// The kit's own theme and a tinted one.
fn identity() -> Identity {
    Identity::new("Kit").themes([
        Theme::new("own", "Own", Palette::own),
        Theme::new("tinted", "Tinted", tinted),
    ])
}

fn appear(theme: &str, shade: Shade, cx: &mut VisualTestContext) {
    let appearance = Appearance {
        source: Source::Theme(theme.to_owned().into()),
        shade,
    };
    cx.update(|_, cx| appearance::appear(appearance, cx));
}

struct Subject {
    text: SharedString,
    shown: bool,
}

impl Render for Subject {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(div().id("before").h(px(20.)).test_support())
            .child(QrCode::new("qr", self.text.clone(), LABEL).shown(self.shown))
    }
}

fn subject<'a>(
    text: &str,
    shown: bool,
    cx: &'a mut TestAppContext,
) -> (Entity<Subject>, &'a mut VisualTestContext) {
    init_as(identity(), cx);
    let text = SharedString::from(text.to_owned());
    let (view, cx) = cx.add_window_view(move |_, _| Subject { text, shown });
    cx.simulate_resize(size(px(800.), px(600.)));
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

fn change(view: &Entity<Subject>, cx: &mut VisualTestContext, f: impl FnOnce(&mut Subject)) {
    cx.update(|_, cx| {
        view.update(cx, |subject, cx| {
            f(subject);
            cx.notify()
        })
    });
    cx.update(|window, cx| window.render_frame(cx));
}

fn bounds(id: &'static str, cx: &mut VisualTestContext) -> gpui_kit::Bounds<gpui_kit::Pixels> {
    cx.update(|window, _| window.find(id).bounds())
}

/// The modules of `text` as the encoder gives them.
fn encoded(text: &str) -> qrcodegen::QrCode {
    qrcodegen::QrCode::encode_text(text, QrCodeEcc::Medium).expect("it encodes")
}

/// The modules the grid really painted, rebuilt from its quads: a square of `side` booleans.
fn painted_modules(side: usize, cx: &mut VisualTestContext) -> Vec<bool> {
    let modules = bounds("qr-modules", cx).size;
    assert_eq!(modules.width, modules.height, "the grid is square");
    let module = f32::from(modules.width) / side as f32;
    assert!(
        module >= 1. && module.fract() == 0.,
        "whole pixels: {module}"
    );
    let ink = seen::colour("qr-modules").expect("the modules have an ink");
    let mut grid = vec![false; side * side];
    for (quad, colour) in qr_code::painted("qr-modules") {
        assert_eq!(colour, ink, "every quad is in the ink");
        assert_eq!(
            f32::from(quad.size.height),
            module,
            "a quad is a module high"
        );
        let (x, y) = (
            f32::from(quad.origin.x) / module,
            f32::from(quad.origin.y) / module,
        );
        let len = f32::from(quad.size.width) / module;
        for value in [x, y, len] {
            assert!(value.fract() == 0., "on the grid of modules: {quad:?}");
        }
        for at in x as usize..(x + len) as usize {
            let cell = &mut grid[y as usize * side + at];
            assert!(!*cell, "module ({at}, {y}) is painted once");
            *cell = true;
        }
    }
    grid
}

fn assert_painted_as_encoded(text: &str, cx: &mut VisualTestContext) {
    let code = encoded(text);
    let side = code.size() as usize;
    let grid = painted_modules(side, cx);
    for y in 0..side {
        for x in 0..side {
            assert_eq!(
                grid[y * side + x],
                code.get_module(x as i32, y as i32),
                "module ({x}, {y}) of {side} × {side}"
            );
        }
    }
}

#[gpui_kit::test]
fn a_qr_code_draws_the_modules_of_its_text(cx: &mut TestAppContext) {
    assert_eq!(INVITE.len(), 180);
    for text in [INVITE, "hello"] {
        let (_, cx) = subject(text, true, cx);
        assert_painted_as_encoded(text, cx);
    }
}

#[gpui_kit::test]
fn a_qr_code_draws_the_modules_of_the_text_it_is_given_now(cx: &mut TestAppContext) {
    let (view, cx) = subject("hello", true, cx);
    change(&view, cx, |subject| subject.text = INVITE.into());
    assert_painted_as_encoded(INVITE, cx);
}

#[gpui_kit::test]
fn a_qr_code_keeps_its_quiet_zone(cx: &mut TestAppContext) {
    let side = encoded(INVITE).size() as usize;
    let (_, cx) = subject(INVITE, true, cx);
    let (plate, modules) = (bounds("qr-plate", cx), bounds("qr-modules", cx));
    let module = modules.size.width / side as f32;
    assert_eq!(plate.size.width, plate.size.height, "the plate is square");
    assert_eq!(
        plate.size.width,
        module * (side + 8) as f32,
        "4 modules of quiet zone on each side"
    );
    for (inset, edge) in [
        (modules.origin.x - plate.origin.x, "left"),
        (modules.origin.y - plate.origin.y, "top"),
        (plate.right() - modules.right(), "right"),
        (plate.bottom() - modules.bottom(), "bottom"),
    ] {
        assert_eq!(inset, module * 4., "the {edge} quiet zone");
    }
}

/// The plate and the modules as last drawn: dark on light, and readable.
fn assert_scannable(what: &str) {
    let plate = seen::colour("qr-plate").expect("the plate is drawn");
    let ink = seen::colour("qr-modules").expect("the modules are drawn");
    assert!(plate.l > 0.5, "{what}: a light plate, not {plate:?}");
    assert!(ink.l < 0.5, "{what}: dark modules, not {ink:?}");
    assert!(
        contrast(plate, ink) >= 7.,
        "{what}: contrast {}",
        contrast(plate, ink)
    );
}

#[gpui_kit::test]
fn a_qr_code_is_scannable_in_both_themes(cx: &mut TestAppContext) {
    let (view, cx) = subject(INVITE, true, cx);
    for (theme, shade) in [
        ("own", Shade::Light),
        ("own", Shade::Dark),
        ("tinted", Shade::Dark),
        ("tinted", Shade::Light),
    ] {
        appear(theme, shade, cx);
        advance(cx, COVER + Duration::from_millis(100));
        change(&view, cx, |_| {});
        assert_scannable(&format!("{theme} {shade:?}"));
    }
}

#[gpui_kit::test]
fn a_qr_code_stays_scannable_while_the_theme_cross_fades(cx: &mut TestAppContext) {
    let (view, cx) = subject(INVITE, true, cx);
    appear("own", Shade::Dark, cx);
    advance(cx, COVER + Duration::from_millis(100));
    appear("own", Shade::Light, cx);
    let mut halfway = 0;
    for step in 0..=(COVER.as_millis() / 16 + 2) {
        advance(cx, Duration::from_millis(16));
        change(&view, cx, |_| {});
        let blending = cx.update(|_, cx| palette(cx).bg != target_palette(cx).bg);
        halfway += usize::from(blending);
        assert_scannable(&format!("frame {step} of the cross-fade"));
    }
    assert!(halfway > 3, "the palette really blended: {halfway} frames");
}

#[gpui_kit::test]
fn a_text_too_long_for_a_qr_code_draws_nothing(cx: &mut TestAppContext) {
    let (view, cx) = subject(&"a".repeat(3000), true, cx);
    let nothing = |cx: &mut VisualTestContext| {
        cx.update(|window, _| {
            ["qr", "qr-code", "qr-plate", "qr-modules"]
                .iter()
                .all(|id| window.try_find(*id).is_none())
                && window.try_find_by_label(LABEL).is_none()
        })
    };
    assert!(nothing(cx), "no element, so no label");
    change(&view, cx, |subject| subject.shown = false);
    change(&view, cx, |subject| subject.shown = true);
    assert_eq!(pending_frames(cx), 0, "and nothing unfolds");
    assert!(nothing(cx));
}

#[gpui_kit::test]
fn a_qr_code_encodes_its_text_once_and_not_on_every_frame(cx: &mut TestAppContext) {
    let (view, cx) = subject(INVITE, false, cx);
    let first = qr_code::encodes();
    assert!(first >= 1);
    change(&view, cx, |subject| subject.shown = true);
    advance(cx, PANEL * 2);
    change(&view, cx, |_| {});
    assert_eq!(qr_code::encodes(), first, "a frame, a fold, a redraw");
    change(&view, cx, |subject| subject.text = "hello".into());
    assert_eq!(qr_code::encodes(), first + 1, "a new text is encoded");
    change(&view, cx, |_| {});
    assert_eq!(qr_code::encodes(), first + 1);
}

#[gpui_kit::test]
fn a_qr_code_has_an_accessible_label(cx: &mut TestAppContext) {
    let (_, cx) = subject(INVITE, true, cx);
    assert_eq!(
        cx.update(|window, _| window.find("qr-code").label().map(str::to_owned)),
        Some(LABEL.to_owned()),
        "it is named by what it holds, not by the text it encodes"
    );
}

/// The height of the block, frame after frame, from the frame the change is drawn in.
fn heights(cx: &mut VisualTestContext) -> Vec<f32> {
    let step = Duration::from_millis(16);
    let height = |cx: &mut VisualTestContext| {
        cx.update(|window, _| {
            window
                .try_find("qr")
                .map_or(0., |found| f32::from(found.bounds().size.height))
        })
    };
    let mut seen = vec![height(cx)];
    let mut gone = Duration::ZERO;
    while gone < PANEL + step * 2 {
        advance(cx, step);
        seen.push(height(cx));
        gone += step;
    }
    seen
}

#[gpui_kit::test]
fn a_qr_code_unfolds_and_fades_in_when_shown(cx: &mut TestAppContext) {
    let (view, cx) = subject(INVITE, false, cx);
    assert!(
        cx.update(|window, _| window.try_find("qr-code").is_none()),
        "absent at rest: nothing is drawn"
    );
    change(&view, cx, |subject| subject.shown = true);
    let grown = heights(cx);
    let full = *grown.last().expect("frames");
    assert!(full >= 100., "it ends at its size: {full}");
    assert!(grown[0] < full / 4., "it starts folded: {grown:?}");
    assert!(
        grown.windows(2).all(|pair| pair[0] <= pair[1]),
        "it only grows: {grown:?}"
    );
    assert!(
        grown.iter().any(|h| 0. < *h && *h < full),
        "it unfolds over time rather than jumps: {grown:?}"
    );
    assert_eq!(pending_frames(cx), 0, "and then asks for no more frames");
}

#[gpui_kit::test]
fn a_qr_code_folds_and_fades_out_when_it_leaves(cx: &mut TestAppContext) {
    let (view, cx) = subject(INVITE, true, cx);
    let full = f32::from(bounds("qr", cx).size.height);
    change(&view, cx, |subject| subject.shown = false);
    let folded = heights(cx);
    assert!(
        folded.iter().any(|h| 0. < *h && *h < full),
        "it folds away over time rather than jumps: {folded:?}"
    );
    assert!(
        folded.windows(2).all(|pair| pair[0] >= pair[1]),
        "{folded:?}"
    );
    assert_eq!(*folded.last().expect("frames"), 0.);
    assert!(cx.update(|window, _| window.try_find("qr-code").is_none()));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn with_reduced_motion_a_qr_code_jumps(cx: &mut TestAppContext) {
    init(cx);
    cx.update(|cx| cx.set_reduce_motion(true));
    let (view, cx) = subject(INVITE, false, cx);
    change(&view, cx, |subject| subject.shown = true);
    advance(cx, Duration::from_millis(16));
    assert!(f32::from(bounds("qr", cx).size.height) >= 100.);
    change(&view, cx, |subject| subject.shown = false);
    advance(cx, Duration::from_millis(16));
    assert!(cx.update(|window, _| window.try_find("qr").is_none()));
}
