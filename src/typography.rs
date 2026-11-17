//! Type: Geist for the interface, Geist Mono for paths, branches, models, tool names and
//! arguments, times, counts, pills and section labels. Both are bundled (SIL OFL 1.1, the licence
//! is beside the files) and set with tabular figures everywhere. The look
//! ([`Fonts`](crate::look::Fonts)) can give the interface, the titles and the controls faces of
//! the application's own.

use std::borrow::Cow;
use std::sync::Arc;

use gpui_kit::{App, Font, FontFeatures, FontWeight, Styled, font, px};

use crate::look::look;

pub const UI: &str = "Geist";
pub const MONO: &str = "Geist Mono";

/// Body text size, in pixels.
pub const BODY: f32 = 13.5;

const FONTS: [&[u8]; 6] = [
    include_bytes!("../assets/fonts/Geist-Regular.ttf"),
    include_bytes!("../assets/fonts/Geist-Medium.ttf"),
    include_bytes!("../assets/fonts/Geist-SemiBold.ttf"),
    include_bytes!("../assets/fonts/GeistMono-Regular.ttf"),
    include_bytes!("../assets/fonts/GeistMono-Medium.ttf"),
    include_bytes!("../assets/fonts/GeistMono-SemiBold.ttf"),
];

pub(crate) fn register(cx: &mut App) {
    let fonts = FONTS.iter().map(|bytes| Cow::Borrowed(*bytes)).collect();
    if let Err(e) = cx.text_system().add_fonts(fonts) {
        crate::identity::log(
            cx,
            format_args!("the bundled fonts could not be loaded: {e}"),
        );
    }
}

/// Registers the files of the application's own faces (TrueType or OpenType), which its look
/// then names by family ([`Fonts`](crate::look::Fonts)). A file that cannot be read is said on the
/// standard error, in the application's name, and the faces fall back to the system's.
pub fn add_fonts(files: impl IntoIterator<Item = Cow<'static, [u8]>>, cx: &mut App) {
    if let Err(e) = cx.text_system().add_fonts(files.into_iter().collect()) {
        crate::identity::log(cx, format_args!("the fonts could not be loaded: {e}"));
    }
}

fn tabular() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".into(), 1)]))
}

/// The faces, applied to any styled element.
pub trait Type: Styled + Sized {
    /// The interface face at body size: the look's ([`Fonts::ui`](crate::look::Fonts::ui)), Geist
    /// by default.
    fn ui_text(self) -> Self {
        self.font_family(look().fonts.ui.clone())
            .font_features(tabular())
            .text_size(px(BODY))
            .line_height(px(BODY * 1.5))
    }

    /// The look's title face, when it gives one; the face around it otherwise.
    fn title_text(self) -> Self {
        match look().fonts.title.clone() {
            Some(face) => self.font_family(face),
            None => self,
        }
    }

    /// The look's control face, when it gives one; the face around it otherwise.
    fn control_face(self) -> Self {
        match look().fonts.control.clone() {
            Some(face) => self.font_family(face),
            None => self,
        }
    }

    /// The interface face at body size in the look's control face: the words of a button, a
    /// segment, a select, a tab, a chip.
    fn control_text(self) -> Self {
        self.ui_text().control_face()
    }

    /// The mono face at `size` pixels.
    fn mono(self, size: f32) -> Self {
        self.font_family(MONO)
            .font_features(tabular())
            .text_size(px(size))
            .line_height(px((size * 1.4).round()))
    }

    fn medium(self) -> Self {
        self.font_weight(FontWeight::MEDIUM)
    }

    fn semibold(self) -> Self {
        self.font_weight(FontWeight::SEMIBOLD)
    }
}

impl<T: Styled> Type for T {}

/// The face a control's words are measured in: the look's control face, else its interface face.
pub(crate) fn control_font() -> Font {
    let look = look();
    font(
        look.fonts
            .control
            .clone()
            .unwrap_or_else(|| look.fonts.ui.clone()),
    )
}
