//! Where the window's colours come from: one of the application's own themes, or the theme of
//! the desktop, followed live.
//!
//! The kit knows no particular desktop. A desktop theme source ([`DesktopTheme`]) says whether it
//! is here, reads the desktop's current palette (or the reason it cannot), and says when it
//! changed; the application registers the ones it offers at [`init`](crate::init)
//! ([`Identity::desktop`](crate::Identity::desktop)). The user chooses ([`appear`]): a desktop
//! source, or one of the application's themes, light or dark. A desktop chosen whose theme cannot
//! be used, the application's first theme in that shade, with the reason logged.

use std::rc::Rc;
use std::time::Duration;

use gpui_kit::{App, Global, SharedString};

use crate::identity::{identity, log};
use crate::theme::{self, Palette, Shade};

/// A desktop's theme, as a source of the window's palette.
pub trait DesktopTheme: std::fmt::Debug + 'static {
    /// What the user's choice keeps to name it (`"dusk"`).
    fn id(&self) -> &'static str;
    /// Its name as a person reads it (`"Dusk"`).
    fn name(&self) -> &'static str;
    /// Whether this machine runs that desktop.
    fn available(&self) -> bool;
    /// The desktop's current palette, or why it cannot be used.
    fn read(&self) -> Result<Palette, String>;
    /// Whether the desktop's theme changed since the last time it was asked. Asked every
    /// [`POLL`] while the source is available: one look must be cheap.
    fn changed(&self) -> bool;
}

/// How often an available desktop source is asked whether its theme changed.
pub const POLL: Duration = Duration::from_secs(1);

/// What a desktop source last gave.
#[derive(Clone, Debug, PartialEq)]
enum Read {
    /// Not this desktop.
    Absent,
    Usable(Box<Palette>),
    /// This desktop, with a theme that cannot be used, and why.
    Unusable(String),
}

/// What each desktop source last gave, and what the user wants.
#[derive(Default)]
struct Looks {
    read: Vec<(&'static str, Read)>,
    /// Nothing chosen yet: the first desktop source, or the first theme, in the dark shade.
    wanted: Option<Appearance>,
}

impl Global for Looks {}

impl Looks {
    fn read(&self, id: &str) -> Option<&Read> {
        self.read
            .iter()
            .find(|(at, _)| *at == id)
            .map(|(_, read)| read)
    }
}

/// Where the window's colours come from, as the user chose it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// A desktop source by its id, followed live.
    Desktop(SharedString),
    /// One of the application's own themes, by its id, in the shade chosen.
    Theme(SharedString),
}

/// The user's choice of the window's colours: a desktop's, or one of the application's own
/// themes, light or dark. The shade is kept with a desktop too: it is the application's own when
/// the desktop's theme cannot be used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Appearance {
    pub source: Source,
    pub shade: Shade,
}

/// The appearance chosen, or the first desktop source (the first theme without one) when nothing
/// is.
fn wanted(cx: &App) -> Appearance {
    if let Some(wanted) = cx
        .try_global::<Looks>()
        .and_then(|looks| looks.wanted.clone())
    {
        return wanted;
    }
    let identity = identity(cx);
    let source = match identity.desktops.first() {
        Some(desktop) => Source::Desktop(desktop.id().into()),
        None => Source::Theme(identity.themes[0].id.clone()),
    };
    Appearance {
        source,
        shade: Shade::default(),
    }
}

/// Uses `appearance` from now on, cross-fading to its palette.
pub fn appear(appearance: Appearance, cx: &mut App) {
    if wanted(cx) == appearance {
        return;
    }
    cx.default_global::<Looks>().wanted = Some(appearance);
    show(cx);
}

/// The appearance the window shows: the one chosen, or the application's first theme in the shade
/// chosen when a desktop is chosen whose theme cannot be used, or when the theme chosen is not one
/// of the application's.
pub fn shown(cx: &App) -> Appearance {
    let wanted = wanted(cx);
    let identity = identity(cx);
    let usable = |id: &str| {
        cx.try_global::<Looks>()
            .and_then(|looks| looks.read(id))
            .is_some_and(|read| matches!(read, Read::Usable(_)))
    };
    let source = match wanted.source {
        Source::Desktop(id) if usable(&id) => Source::Desktop(id),
        Source::Desktop(_) => Source::Theme(identity.themes[0].id.clone()),
        Source::Theme(id) => Source::Theme(identity.theme(&id).id.clone()),
    };
    Appearance {
        source,
        shade: wanted.shade,
    }
}

/// Cross-fades to the palette of what is shown.
fn show(cx: &mut App) {
    let shown = shown(cx);
    let palette = match &shown.source {
        Source::Desktop(id) => match cx.default_global::<Looks>().read(id) {
            Some(Read::Usable(palette)) => **palette,
            // Not shown: a desktop is shown only when its theme can be used.
            _ => Palette::base(),
        },
        Source::Theme(id) => (identity(cx).theme(id).palette)(shown.shade),
    };
    theme::set_palette(palette, cx);
}

/// Why the desktop source `id` cannot be chosen here: not this desktop, or a theme that cannot be
/// used. `None` when it can.
pub fn refused(id: &str, cx: &App) -> Option<String> {
    let identity = identity(cx);
    let app = identity.name;
    let name = identity
        .desktops
        .iter()
        .find(|desktop| desktop.id() == id)
        .map_or(id, |desktop| desktop.name());
    match cx.try_global::<Looks>().and_then(|looks| looks.read(id)) {
        Some(Read::Usable(_)) => None,
        Some(Read::Unusable(reason)) => Some(format!(
            "The {name} theme cannot be used ({reason}): the window uses {app}'s colours."
        )),
        Some(Read::Absent) | None => Some(format!(
            "No {name} theme on this desktop: the window uses {app}'s colours."
        )),
    }
}

/// The last reason the theme of the desktop source `id` was set aside, kept beside the log line
/// for whoever asks.
pub fn fallback(id: &str, cx: &App) -> Option<String> {
    match cx.try_global::<Looks>().and_then(|looks| looks.read(id)) {
        Some(Read::Unusable(reason)) => Some(reason.clone()),
        _ => None,
    }
}

fn read(desktop: &dyn DesktopTheme) -> Read {
    if !desktop.available() {
        return Read::Absent;
    }
    desktop
        .read()
        .map_or_else(Read::Unusable, |palette| Read::Usable(Box::new(palette)))
}

/// Keeps what `desktop` gave now, and cross-fades to what is shown.
fn apply(desktop: &dyn DesktopTheme, read: Read, cx: &mut App) {
    if let Read::Unusable(reason) = &read {
        log(
            cx,
            format_args!(
                "the {} theme is not used, the base theme is: {reason}",
                desktop.name()
            ),
        );
    }
    let looks = cx.default_global::<Looks>();
    looks.read.retain(|(id, _)| *id != desktop.id());
    looks.read.push((desktop.id(), read));
    show(cx);
}

/// Reads every desktop source of the application, shows what the user wants, and keeps asking the
/// available sources whether their theme changed.
pub(crate) fn install(cx: &mut App) {
    let desktops = identity(cx).desktops;
    for desktop in &desktops {
        let read = read(desktop.as_ref());
        apply(desktop.as_ref(), read, cx);
    }
    show(cx);
    let followed: Vec<Rc<dyn DesktopTheme>> = desktops
        .into_iter()
        .filter(|desktop| desktop.available())
        .collect();
    if followed.is_empty() {
        return;
    }
    cx.spawn(async move |cx| {
        loop {
            cx.background_executor().timer(POLL).await;
            for desktop in &followed {
                if desktop.changed() {
                    let read = read(desktop.as_ref());
                    cx.update(|cx| apply(desktop.as_ref(), read, cx));
                }
            }
        }
    })
    .detach();
}
