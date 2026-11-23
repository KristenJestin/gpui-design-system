//! `MachineMark`: a machine's icon in its own colour with a badge on its corner, one shape per
//! state, never told by the machine's colour; on a cut-out of the surface behind it; its words its
//! tooltip and its accessible label; a state that changes fades and scales the badge and moves the
//! icon's colour over `PANEL`, and with reduced motion jumps. On GPUI's test platform.

mod common;

use common::{advance, init, pending_frames};
use gpui_design_system::appearance::{self, Appearance, Source};
use gpui_design_system::color::hex;
use gpui_design_system::motion::{COVER, HOVER, PANEL, TOOLTIP};
use gpui_design_system::seen::{colour, seen};
use gpui_design_system::theme::{Shade, palette};
use gpui_design_system::{Icon, MachineMark, MachineState, MarkSize};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Bounds, Context, Entity, Hsla, IntoElement, ParentElement, Pixels, Render, SharedString,
    Styled, TestAppContext, VisualTestContext, Window, div, px, size,
};

const STATES: [(MachineState, &str); 5] = [
    (MachineState::Here, "here"),
    (MachineState::Online, "online"),
    (MachineState::Offline, "offline"),
    (MachineState::Update, "update"),
    (MachineState::Unknown, "unknown"),
];

const SIZES: [(MarkSize, f32); 3] = [
    (MarkSize::Large, 16.),
    (MarkSize::Medium, 14.),
    (MarkSize::Small, 12.),
];

struct Marked {
    state: MachineState,
    size: MarkSize,
    machine: Hsla,
    surface: Option<Hsla>,
    label: SharedString,
}

impl Render for Marked {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut mark = MachineMark::new(
            "mark",
            Icon::Lucide("server"),
            self.machine,
            self.state,
            self.label.clone(),
        )
        .size(self.size);
        if let Some(surface) = self.surface {
            mark = mark.on(surface);
        }
        // Room on every side: the badge overhangs the icon's box.
        div().p(px(40.)).child(mark)
    }
}

fn teal() -> Hsla {
    hex(0x2dd4bf)
}

fn marked(
    state: MachineState,
    size: MarkSize,
    reduce: bool,
    cx: &mut TestAppContext,
) -> (Entity<Marked>, &mut VisualTestContext) {
    init(cx);
    cx.update(|cx| cx.set_reduce_motion(reduce));
    let (view, cx) = cx.add_window_view(move |_, _| Marked {
        state,
        size,
        machine: teal(),
        surface: None,
        label: "server · online".into(),
    });
    cx.simulate_resize(size_of_window());
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

fn size_of_window() -> gpui_kit::Size<Pixels> {
    size(px(300.), px(200.))
}

fn change(
    view: &Entity<Marked>,
    cx: &mut VisualTestContext,
    to: impl FnOnce(&mut Marked) + 'static,
) {
    cx.update(|_, cx| {
        view.update(cx, |marked, cx| {
            to(marked);
            cx.notify()
        })
    });
    cx.update(|window, cx| window.render_frame(cx));
}

fn bounds(id: &str, cx: &mut VisualTestContext) -> Bounds<Pixels> {
    cx.update(|window, _| window.find(id.to_owned()).bounds())
}

fn drawn(id: &str, cx: &mut VisualTestContext) -> bool {
    cx.update(|window, _| window.try_find(id.to_owned()).is_some())
}

fn side(bounds: Bounds<Pixels>) -> f32 {
    f32::from(bounds.size.width)
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.05
}

#[gpui_kit::test]
fn every_state_at_every_size_draws_its_own_shape_and_no_other(cx: &mut TestAppContext) {
    for (size, px_size) in SIZES {
        for (state, name) in STATES {
            let (_, cx) = marked(state, size, false, cx);
            let mark = bounds("mark", cx);
            assert!(near(side(mark), px_size), "{name} at {px_size}: {mark:?}");
            let badge = bounds("mark-badge", cx);
            let box_side = px_size * 0.58;
            assert!(
                near(side(badge), box_side),
                "{name} at {px_size}: the badge is {box_side} px, not {}",
                side(badge)
            );
            // On the icon's bottom-right corner, hanging out of it.
            assert!(near(
                f32::from(badge.right() - mark.right()),
                px_size * 0.24
            ));
            assert!(near(
                f32::from(badge.bottom() - mark.bottom()),
                px_size * 0.24
            ));
            for (other, shape) in STATES {
                let there = drawn(&format!("mark-{shape}"), cx);
                assert_eq!(
                    there,
                    other == state,
                    "{name} at {px_size}: the {shape} shape is {}",
                    if there { "drawn" } else { "missing" }
                );
            }
            let shape = bounds(&format!("mark-{name}"), cx);
            assert!(near(side(shape), box_side), "{name} at {px_size}");
            assert_eq!(pending_frames(cx), 0, "a mark at rest draws no frame");
        }
    }
}

#[gpui_kit::test]
fn a_shape_is_drawn_in_the_palette_s_colour_of_its_state(cx: &mut TestAppContext) {
    let p = |cx: &mut VisualTestContext| cx.update(|_, cx| palette(cx));
    for (state, name, ink) in [
        (
            MachineState::Here,
            "here",
            (|p: gpui_design_system::Palette| p.text) as fn(_) -> Hsla,
        ),
        (MachineState::Online, "online", |p| p.ok),
        (MachineState::Offline, "offline", |p| p.faint),
        (MachineState::Update, "update", |p| p.wait),
        (MachineState::Unknown, "unknown", |p| p.faint),
    ] {
        let (_, cx) = marked(state, MarkSize::Large, false, cx);
        let palette = p(cx);
        assert_eq!(colour(format!("mark-{name}")), Some(ink(palette)), "{name}");
    }
}

#[gpui_kit::test]
fn the_machine_s_colour_never_says_its_state(cx: &mut TestAppContext) {
    // A machine whose own colour is the green of "online": its other states do not borrow it.
    let green = cx.update(|cx| palette(cx)).ok;
    for (state, name) in STATES {
        let (view, cx) = marked(state, MarkSize::Large, false, cx);
        change(&view, cx, move |marked| marked.machine = green);
        let palette = cx.update(|_, cx| palette(cx));
        let badge = colour(format!("mark-{name}")).unwrap();
        match state {
            MachineState::Online => assert_eq!(badge, palette.ok),
            _ => assert_ne!(badge, green, "{name} is not told by the machine's green"),
        }
        let icon = colour("mark-icon").unwrap();
        match state {
            MachineState::Offline => assert_eq!(icon, palette.faint, "offline greys the icon"),
            MachineState::Unknown => {
                assert_eq!(icon.h, green.h);
                assert!(icon.a < 1.0, "unknown fades the icon a little");
            }
            _ => assert_eq!(icon, green, "{name} keeps the machine's colour"),
        }
    }
}

#[gpui_kit::test]
fn the_badge_sits_on_a_cut_out_of_the_surface_behind_it(cx: &mut TestAppContext) {
    let row = hex(0x223344);
    let (view, cx) = marked(MachineState::Online, MarkSize::Large, false, cx);
    let badge = bounds("mark-badge", cx);
    let cut = bounds("mark-cutout", cx);
    assert!(
        near(side(cut), side(badge) + 2.8),
        "{cut:?} around {badge:?}"
    );
    assert!(near(f32::from(badge.left() - cut.left()), 1.4));
    assert_eq!(
        colour("mark-cutout"),
        Some(cx.update(|_, cx| palette(cx)).side),
        "the left tree's colour unless it is told another"
    );

    change(&view, cx, move |marked| marked.surface = Some(row));
    assert!(
        pending_frames(cx) > 0,
        "a row that is hovered or chosen fades the cut-out"
    );
    advance(cx, HOVER / 3);
    let partway = colour("mark-cutout").unwrap();
    assert_ne!(partway, row, "it is on its way");
    advance(cx, HOVER * 2);
    assert_eq!(colour("mark-cutout"), Some(row));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn its_words_are_its_accessible_label_and_its_tooltip(cx: &mut TestAppContext) {
    let (view, cx) = marked(MachineState::Offline, MarkSize::Medium, false, cx);
    change(&view, cx, |marked| {
        marked.label = "server · offline since 09:33".into()
    });
    cx.update(|window, _| {
        assert_eq!(
            window.find("mark").label(),
            Some("server · offline since 09:33")
        )
    });
    assert!(
        !drawn("mark-tooltip", cx),
        "no tooltip before the pointer rests"
    );
    cx.update(|window, cx| window.hover("mark", cx));
    advance(cx, TOOLTIP + HOVER * 2);
    cx.update(|window, cx| {
        window.render_frame(cx);
        assert_eq!(
            window.find("mark-tooltip").label(),
            Some("server · offline since 09:33")
        )
    });
}

#[gpui_kit::test]
fn a_state_that_changes_fades_and_scales_the_badge_over_panel(cx: &mut TestAppContext) {
    let (view, cx) = marked(MachineState::Online, MarkSize::Large, false, cx);
    let full = side(bounds("mark-online", cx));
    change(&view, cx, |marked| marked.state = MachineState::Offline);
    assert!(pending_frames(cx) > 0, "the badge is changing");
    advance(cx, PANEL / 4);
    cx.update(|window, cx| window.render_frame(cx));
    assert!(
        drawn("mark-online", cx) && drawn("mark-offline", cx),
        "both on the way"
    );
    let (going, coming) = (
        side(bounds("mark-online", cx)),
        side(bounds("mark-offline", cx)),
    );
    assert!(
        going < full && going > 0.,
        "the old shape shrinks: {going} of {full}"
    );
    assert!(
        coming > 0. && coming < full,
        "the new one grows: {coming} of {full}"
    );
    let (fading, rising) = (seen("mark-online").unwrap(), seen("mark-offline").unwrap());
    assert!(fading < 1. && fading > 0. && rising > 0. && rising < 1.);
    advance(cx, PANEL * 2);
    cx.update(|window, cx| window.render_frame(cx));
    assert!(!drawn("mark-online", cx), "the old shape is gone");
    assert!(near(side(bounds("mark-offline", cx)), full));
    assert_eq!(pending_frames(cx), 0, "it stopped once it was there");
}

#[gpui_kit::test]
fn a_state_that_changes_moves_the_icon_s_colour_over_panel(cx: &mut TestAppContext) {
    let (view, cx) = marked(MachineState::Online, MarkSize::Large, false, cx);
    assert_eq!(colour("mark-icon"), Some(teal()));
    change(&view, cx, |marked| marked.state = MachineState::Offline);
    advance(cx, PANEL / 4);
    let partway = colour("mark-icon").unwrap();
    let grey = cx.update(|_, cx| palette(cx)).faint;
    assert!(
        partway != teal() && partway != grey,
        "on its way: {partway:?}"
    );
    advance(cx, PANEL * 2);
    assert_eq!(colour("mark-icon"), Some(grey));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_mark_first_drawn_in_a_state_is_simply_there(cx: &mut TestAppContext) {
    let (_, cx) = marked(MachineState::Update, MarkSize::Small, false, cx);
    assert_eq!(seen("mark-update"), Some(1.0));
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn with_reduced_motion_a_change_of_state_jumps(cx: &mut TestAppContext) {
    let (view, cx) = marked(MachineState::Online, MarkSize::Large, true, cx);
    let full = side(bounds("mark-online", cx));
    change(&view, cx, |marked| marked.state = MachineState::Unknown);
    assert_eq!(pending_frames(cx), 0, "nothing is animated");
    assert!(!drawn("mark-online", cx), "the old shape is already gone");
    assert!(near(side(bounds("mark-unknown", cx)), full));
    let faint = cx.update(|_, cx| palette(cx)).faint;
    assert_eq!(colour("mark-unknown"), Some(faint));
    assert_eq!(colour("mark-icon").map(|c| c.h), Some(teal().h));
}

#[gpui_kit::test]
fn both_themes_draw_the_badge_in_their_own_colours(cx: &mut TestAppContext) {
    let (_, cx) = marked(MachineState::Online, MarkSize::Large, false, cx);
    let mut seen_ok = Vec::new();
    for shade in [Shade::Dark, Shade::Light] {
        cx.update(|_, cx| {
            appearance::appear(
                Appearance {
                    source: Source::Theme("Kit".into()),
                    shade,
                },
                cx,
            )
        });
        advance(cx, COVER * 2);
        cx.update(|window, cx| window.render_frame(cx));
        let ok = cx.update(|_, cx| palette(cx)).ok;
        assert_eq!(colour("mark-online"), Some(ok), "{shade:?}");
        seen_ok.push(ok);
    }
    assert_ne!(
        seen_ok[0], seen_ok[1],
        "the two themes have their own green"
    );
}
