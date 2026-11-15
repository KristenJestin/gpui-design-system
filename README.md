# gpui-design-system

The components, tokens and motion shared by applications built on GPUI through
[gpui-kit](https://crates.io/crates/gpui-kit) and GPUI Fast: buttons, rows, cards, menus,
dialogs, drawers, toasts, fields, tabs, skeletons, the palette every screen reads, and the motion
they share. Every screen of an application is built from these pieces; a second, slightly
different version of one of them is a defect.

The kit knows nothing of what an application is about. The application tells it its name, its
themes and the desktops whose theme it offers to follow when it starts.

Bootstrapped from the `flottille-ui` crate of
[Flottille](https://github.com/KristenJestin/flottille).

## Depending on it

A git dependency, pinned to a tag:

```toml
[dependencies]
gpui-design-system = { git = "https://github.com/KristenJestin/gpui-design-system", tag = "v0.1.0" }
gpui-kit = { version = "=0.8.0", default-features = false, features = ["assets", "gpui-fast"] }
```

Every version is a tag that never moves, with a release and its changelog: `vX.Y.Z` for a stable
release, `vX.Y.Z-beta.N` for each commit of `main`, `vX.Y.Z-alpha.<branch>.N` for each push of a
feature branch (deleted with the branch). Pin a stable tag; take a beta or an alpha only while
trying something. See [`docs/releases.md`](docs/releases.md).

The kit is built on `gpui-kit =0.8.0` with GPUI Fast. Keep `gpui-fast` at 0.1.10 in your
`Cargo.lock` (`cargo update -p gpui-fast --precise 0.1.10`): 0.1.11 breaks `gpui-base` 0.8.

### Features

- `omarchy` (on by default): the current theme of the Omarchy desktop as a desktop source
  (`omarchy::Omarchy`), followed live. With `default-features = false`, nothing in the kit reads
  Omarchy's files or names it.

## Starting it

```rust
use gpui_design_system::omarchy::Omarchy;
use gpui_design_system::theme::Shade;
use gpui_design_system::{Icons, Identity, Palette, Theme};

/// A look of the application's own: the kit's with a teal primary.
fn teal(shade: Shade) -> Palette {
    let mut palette = Palette::own(shade);
    palette.acc = gpui_design_system::color::hex(0x2bb3a0);
    palette
}

fn main() {
    gpui_kit::application().with_assets(Icons).run(|cx| {
        let identity = Identity::new("Notes")
            .themes([
                Theme::new("plain", "Plain", Palette::own),
                Theme::new("teal", "Teal", teal),
            ])
            .desktop(Omarchy::from_system());
        gpui_design_system::init(identity, cx);
        // Open the windows.
    });
}
```

- **Its name** starts the lines the kit logs and is said in the reasons it gives.
- **Its themes**: each has an id, a name a person reads and a palette in each shade
  (`fn(Shade) -> Palette`). Without `.themes(..)`, the application has one: the kit's own look,
  under its name.
- **Its desktop sources**: each is a `DesktopTheme` (a name, whether it is here, how it reads the
  current palette or why it cannot, and when it changed). The first one is followed until the user
  chooses otherwise; with none, the first theme is shown.

The user's choice is an `Appearance`: a desktop source or a theme, by its id, and a shade.
`appearance::appear` switches to it with a cross-fade; `appearance::shown` says what is shown
(the application's first theme when the desktop's cannot be used); `appearance::refused` says why
a desktop cannot be chosen here. Keeping the choice between two runs is the application's job.

## Icons

`Icons` serves the Lucide icons the components use (the base set of `Icon`). An application that
needs more declares them itself: another Lucide icon by its name (`Icon::Lucide("ticket")`),
embedded with `gpui_kit::assets::icon_assets!`, or a glyph of its own (`Icon::Asset(path)`
stroked, `Icon::Filled(path)` solid). Its assets serve those files and fall back to `Icons`:

```rust
use std::borrow::Cow;

use gpui_design_system::{Icon, Icons};
use gpui_kit::{AssetSource, Result, SharedString};

pub const TICKET: Icon = Icon::Lucide("ticket");

gpui_kit::assets::icon_assets!(Lucide, [Ticket]);

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match Lucide.load(path)? {
            Some(found) => Ok(Some(found)),
            None => Icons.load(path),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = Icons.list(path)?;
        paths.extend(Lucide.list(path)?);
        Ok(paths)
    }
}
```

## Tokens in a text

`TokenField` is a multi-line text field that holds tokens where they were put: a chip (an
optional glyph, a label, an optional detail) with an id of the application's. The caret steps
over one, Backspace or Delete removes it whole, a copy keeps it (pasted elsewhere, it is its
label). `TokenText` draws the same chips in a text that is only read, and says which one is
clicked.

```rust
use gpui_design_system::{Icon, Piece, Token, TokenEvent, TokenField, TokenLook, TokenText};

let field = cx.new(|cx| TokenField::new("draft", window, cx));
cx.subscribe(&field, |_, _, event: &TokenEvent, _| match event {
    TokenEvent::Removed(id) => { /* the person removed it: drop what it stands for */ }
    TokenEvent::Added(id) => { /* a paste or an undo brought it back */ }
})
.detach();

// A picture pasted: its chip at the caret.
let picture = Token::new("picture-1", TokenLook::new("Image 1").glyph(Icon::File).detail("photo.png · 2 MB"));
field.update(cx, |field, cx| field.insert(picture, window, cx))?;

// Sent: the text and the tokens in their order.
let pieces: Vec<Piece> = field.read(cx).value(cx);
// Shown in the conversation, its chips clicked.
TokenText::new("message-1", pieces).on_token_click(|id, window, cx| { /* open it */ });
```

The events are per id, not per copy: `Added(id)` when the first copy of an id comes into the
text, `Removed(id)` when its last copy leaves it. What the application does itself (`insert`,
`remove`, `relabel`, `set_value`) is not said back to it, except the tokens an `insert` replaces over a
selection, which are said `Removed`. A copy with tokens pasted into another field brings its chips
there, said `Added` there; the same words copied by another application stay text.

`relabel(id, look)` gives the chips of an id a new look in place (a list of pictures numbered
again once one is removed): no undo step, no `Change`, the caret and the selection stay, and the
chip's ink fades out and back in over `HOVER`, so the person's undo still reaches their removal.
The text the input holds keeps the label the token was put with. The field's own look wins: a
paste (a copy made before the relabel) keeps the look the field holds for the id, and only brings
the look of an id the field does not know.

## The rules the kit holds

Its tests are its contract: GPUI's own `.hover(..)` is refused (every colour fades), everything
that moves is animated with the same durations and easing, overlays stack and close as one, long
texts are cut with their whole text at hand, and a scan keeps the sources from naming what an
application is about.

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --no-default-features
cargo run --example gallery
cargo run --example components
cargo run --example list_parts
```

The tests run on GPUI's test platform and open no window.

## Licence

AGPL-3.0-only (see `LICENSE`). The bundled Geist and Geist Mono fonts are under the SIL Open Font
License (`assets/fonts/OFL.txt`).
