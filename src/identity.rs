//! What an application tells the kit about itself at [`init`](crate::init): its name, which the
//! kit says in the lines it logs and the reasons it gives, and its own themes.

use gpui_kit::{App, Global};

use crate::theme::{Palette, Theme};

/// The application using the kit.
#[derive(Clone, Debug)]
pub struct Identity {
    /// Its name as a person reads it (`"Notes"`); lowercased, it starts the lines the kit logs.
    pub name: &'static str,
    /// Its own themes, never empty: what is shown when the desktop's theme is not followed. The
    /// first is shown when the one chosen is not among them.
    pub themes: Vec<Theme>,
}

impl Identity {
    /// An application called `name`, with one theme: the kit's own look ([`Palette::own`]), under
    /// the application's name.
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            themes: vec![Theme::new("own", name, Palette::own)],
        }
    }

    /// The same application with the themes it gives itself, in the order it offers them.
    ///
    /// # Panics
    ///
    /// When `themes` is empty: an application shows at least one.
    pub fn themes(mut self, themes: impl IntoIterator<Item = Theme>) -> Self {
        self.themes = themes.into_iter().collect();
        assert!(!self.themes.is_empty(), "an application gives a theme");
        self
    }

    /// Its theme called `id`, or its first when it gives none of that id.
    pub fn theme(&self, id: &str) -> &Theme {
        self.themes
            .iter()
            .find(|theme| theme.id == id)
            .unwrap_or(&self.themes[0])
    }
}

impl Global for Identity {}

/// The application's identity, as given to [`init`](crate::init).
pub fn identity(cx: &App) -> Identity {
    cx.try_global::<Identity>()
        .cloned()
        .unwrap_or_else(|| Identity::new("app"))
}

/// A line for the standard error, in the application's name.
pub(crate) fn log(cx: &App, line: impl std::fmt::Display) {
    eprintln!("{}: {line}", identity(cx).name.to_lowercase());
}
