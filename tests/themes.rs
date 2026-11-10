//! An application gives several themes, and the user's choice names one of them by its id.

mod common;

use std::time::Duration;

use common::{Desktop, init_as};
use gpui_design_system::color::hex;
use gpui_design_system::omarchy::{self, Appearance, Source};
use gpui_design_system::theme::{Shade, palette, target_palette};
use gpui_design_system::{Identity, Palette, Theme};
use gpui_kit::TestAppContext;

/// A theme of invented colours: the kit's own with an orange primary.
fn ember(shade: Shade) -> Palette {
    let mut palette = Palette::own(shade);
    palette.acc = hex(0xf0883e);
    palette
}

/// Two themes: the kit's own look first, then an orange one.
fn notes() -> Identity {
    Identity::new("Notes").themes([
        Theme::new("plain", "Plain", Palette::own),
        Theme::new("ember", "Ember", ember),
    ])
}

fn theme(id: &'static str, shade: Shade) -> Appearance {
    Appearance {
        source: Source::Theme(id.into()),
        shade,
    }
}

fn appear(appearance: Appearance, cx: &mut TestAppContext) {
    cx.update(|cx| omarchy::appear(appearance, cx));
    cx.run_until_parked();
}

#[gpui_kit::test]
fn an_application_with_two_themes_switches_between_them_with_the_same_cross_fade(
    cx: &mut TestAppContext,
) {
    init_as(notes(), cx, Desktop::new(false).env());
    // Nothing chosen and no desktop theme: the first of the application's themes.
    assert_eq!(
        cx.update(|cx| target_palette(cx)),
        Palette::own(Shade::Dark)
    );

    appear(theme("ember", Shade::Dark), cx);
    assert_eq!(cx.update(|cx| target_palette(cx)), ember(Shade::Dark));
    assert_eq!(
        cx.update(|cx| omarchy::shown(cx)),
        theme("ember", Shade::Dark)
    );
    // Partway, the primary sits between the two themes': no jump.
    cx.executor().advance_clock(Duration::from_millis(100));
    let partway = cx.update(|cx| palette(cx)).acc;
    assert_ne!(partway, Palette::own(Shade::Dark).acc);
    assert_ne!(partway, ember(Shade::Dark).acc);
    cx.executor().advance_clock(Duration::from_secs(1));
    cx.run_until_parked();
    assert_eq!(cx.update(|cx| palette(cx)), ember(Shade::Dark));

    appear(theme("ember", Shade::Light), cx);
    assert_eq!(cx.update(|cx| target_palette(cx)), ember(Shade::Light));
    appear(theme("plain", Shade::Light), cx);
    cx.executor().advance_clock(Duration::from_millis(100));
    assert_ne!(cx.update(|cx| palette(cx)), Palette::light(), "it fades");
    cx.executor().advance_clock(Duration::from_secs(1));
    cx.run_until_parked();
    assert_eq!(cx.update(|cx| palette(cx)), Palette::light());
}

#[gpui_kit::test]
fn a_theme_the_application_no_longer_gives_shows_its_first_one(cx: &mut TestAppContext) {
    init_as(notes(), cx, Desktop::new(false).env());
    appear(theme("gone", Shade::Light), cx);
    assert_eq!(cx.update(|cx| target_palette(cx)), Palette::light());
    assert_eq!(
        cx.update(|cx| omarchy::shown(cx)),
        theme("plain", Shade::Light)
    );
}

#[gpui_kit::test]
fn an_application_that_gives_no_theme_has_the_kit_s_own_under_its_name(cx: &mut TestAppContext) {
    let identity = Identity::new("Notes");
    assert_eq!(identity.themes.len(), 1);
    assert_eq!(identity.themes[0].name.as_ref(), "Notes");
    init_as(identity, cx, Desktop::new(false).env());
    assert_eq!(cx.update(|cx| target_palette(cx)), Palette::base());
}
