//! A desktop theme source an application registers at `init`: followed live when the user chooses
//! it, set aside with the reason when it cannot be used. The source here is a fake one, told what
//! to give by the test.

mod common;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use common::init_as;
use gpui_design_system::appearance::{self, Appearance, DesktopTheme, POLL, Source};
use gpui_design_system::color::hex;
use gpui_design_system::theme::{Shade, palette, target_palette};
use gpui_design_system::{Identity, Palette};
use gpui_kit::TestAppContext;

/// A desktop the test drives: whether it is here, what it reads, and whether it changed.
#[derive(Clone, Debug)]
struct Fake {
    here: bool,
    reads: Rc<RefCell<Result<Palette, String>>>,
    changed: Rc<Cell<bool>>,
}

impl Fake {
    fn new(here: bool, reads: Result<Palette, String>) -> Self {
        Self {
            here,
            reads: Rc::new(RefCell::new(reads)),
            changed: Rc::default(),
        }
    }

    /// The desktop switches to `reads`, and says so at the next look.
    fn switch(&self, reads: Result<Palette, String>) {
        *self.reads.borrow_mut() = reads;
        self.changed.set(true);
    }
}

impl DesktopTheme for Fake {
    fn id(&self) -> &'static str {
        "dusk"
    }

    fn name(&self) -> &'static str {
        "Dusk"
    }

    fn available(&self) -> bool {
        self.here
    }

    fn read(&self) -> Result<Palette, String> {
        self.reads.borrow().clone()
    }

    fn changed(&self) -> bool {
        self.changed.replace(false)
    }
}

/// A desktop's palette of invented colours: the kit's own with a teal primary.
fn teal(acc: u32) -> Palette {
    let mut palette = Palette::base();
    palette.acc = hex(acc);
    palette
}

fn notes(desktop: &Fake) -> Identity {
    Identity::new("Notes").desktop(desktop.clone())
}

fn dusk(shade: Shade) -> Appearance {
    Appearance {
        source: Source::Desktop("dusk".into()),
        shade,
    }
}

fn own(shade: Shade) -> Appearance {
    Appearance {
        source: Source::Theme("own".into()),
        shade,
    }
}

fn appear(appearance: Appearance, cx: &mut TestAppContext) {
    cx.update(|cx| appearance::appear(appearance, cx));
    cx.run_until_parked();
}

fn look_again(cx: &mut TestAppContext) {
    cx.executor()
        .advance_clock(POLL + Duration::from_millis(10));
    cx.run_until_parked();
}

#[gpui_kit::test]
fn a_registered_desktop_source_is_shown_by_default_and_followed_live(cx: &mut TestAppContext) {
    let desktop = Fake::new(true, Ok(teal(0x2bb3a0)));
    init_as(notes(&desktop), cx);
    assert_eq!(cx.update(|cx| appearance::shown(cx)), dusk(Shade::Dark));
    assert_eq!(cx.update(|cx| palette(cx)), teal(0x2bb3a0));
    assert!(cx.update(|cx| appearance::refused("dusk", cx)).is_none());

    desktop.switch(Ok(teal(0x7aa2f7)));
    look_again(cx);
    assert_eq!(cx.update(|cx| target_palette(cx)), teal(0x7aa2f7));
    cx.executor().advance_clock(Duration::from_secs(1));
    cx.run_until_parked();
    assert_eq!(cx.update(|cx| palette(cx)), teal(0x7aa2f7));
}

#[gpui_kit::test]
fn a_desktop_not_here_is_refused_and_the_first_theme_is_shown(cx: &mut TestAppContext) {
    let desktop = Fake::new(false, Ok(teal(0x2bb3a0)));
    init_as(notes(&desktop), cx);
    assert_eq!(cx.update(|cx| target_palette(cx)), Palette::base());
    assert_eq!(cx.update(|cx| appearance::shown(cx)), own(Shade::Dark));
    let why = cx
        .update(|cx| appearance::refused("dusk", cx))
        .expect("Dusk cannot be chosen");
    assert!(
        why.contains("Dusk") && why.contains("Notes's colours"),
        "{why}"
    );
    assert!(cx.update(|cx| appearance::fallback("dusk", cx)).is_none());
}

#[gpui_kit::test]
fn a_desktop_theme_that_cannot_be_read_falls_back_with_the_reason(cx: &mut TestAppContext) {
    let desktop = Fake::new(true, Err("no `accent`".into()));
    init_as(notes(&desktop), cx);
    assert_eq!(cx.update(|cx| target_palette(cx)), Palette::base());
    assert_eq!(cx.update(|cx| appearance::shown(cx)), own(Shade::Dark));
    let reason = cx.update(|cx| appearance::fallback("dusk", cx));
    assert_eq!(reason.as_deref(), Some("no `accent`"));
    let why = cx.update(|cx| appearance::refused("dusk", cx)).unwrap();
    assert!(why.contains("no `accent`"), "{why}");

    // Mended on the desktop: followed again, without the user choosing anything.
    desktop.switch(Ok(teal(0x2bb3a0)));
    look_again(cx);
    assert_eq!(cx.update(|cx| target_palette(cx)), teal(0x2bb3a0));
    assert!(cx.update(|cx| appearance::fallback("dusk", cx)).is_none());
}

#[gpui_kit::test]
fn the_own_theme_chosen_stops_following_the_desktop_until_it_is_chosen_again(
    cx: &mut TestAppContext,
) {
    let desktop = Fake::new(true, Ok(teal(0x2bb3a0)));
    init_as(notes(&desktop), cx);
    appear(own(Shade::Light), cx);
    assert_eq!(cx.update(|cx| target_palette(cx)), Palette::light());
    desktop.switch(Ok(teal(0x7aa2f7)));
    look_again(cx);
    assert_eq!(cx.update(|cx| target_palette(cx)), Palette::light());

    appear(dusk(Shade::Light), cx);
    assert_eq!(cx.update(|cx| target_palette(cx)), teal(0x7aa2f7));
}

#[gpui_kit::test]
fn without_a_desktop_source_the_first_theme_is_shown_and_any_desktop_is_refused(
    cx: &mut TestAppContext,
) {
    init_as(Identity::new("Notes"), cx);
    assert_eq!(cx.update(|cx| appearance::shown(cx)), own(Shade::Dark));
    appear(dusk(Shade::Light), cx);
    assert_eq!(cx.update(|cx| target_palette(cx)), Palette::light());
    assert!(cx.update(|cx| appearance::refused("dusk", cx)).is_some());
}
