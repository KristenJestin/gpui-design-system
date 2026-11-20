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

## The look

The faces, the corners, the borders, the shadow and the sizes of the kit are its look, given once
with `init_with(identity, look, cx)` in place of `init(identity, cx)`. Every field defaults to
the kit's own design: `Look::default()` is what `init` draws, exactly as before. The colours stay
the theme's, a desktop source's included.

| Field | What it gives | Default |
|---|---|---|
| `fonts: Fonts { ui, title, control }` | the families of the interface (`Type::ui_text`), of the titles (`title_text`) and of the controls' words (`control_text`: buttons, segmented controls, selects, tabs, chips); `None` takes the interface's | `"Geist"`, `None`, `None` |
| `corners: Corners` | `Rounded` (the kit's radii), `Square`, or `Notch { step, steps }`: each corner cut by `steps` stairs of `step` px, the border following them | `Rounded` |
| `border: Pixels` | the width of every border and separating line | `px(1.)` |
| `shadow: Option<Shadow { x, y, color }>` | a hard shadow, offset and unblurred, under the buttons and the overlays, in the colour of a role of the palette (`look::Role`: `Shadow`, the overlays' shadow colour at full strength, `Line`, `Line2`, `Text`, `Faint` or `Accent`), so it follows the theme; a pressed button moves onto it over `PRESS` (at once with reduced motion) instead of shrinking | `None` |
| `grid: Option<Pixels>` | the unit the look's sizes, paddings and shadow offset are whole multiples of, rounded to the nearest | `None` |
| `sizes: Sizes` | the heights of the controls and the space beside their words: `button` 30, `button_padding` 12, `icon_button` 28, `float_button` 32, `field` 34, `field_padding` 10, `pill` 24, `pill_padding` 9, `segmented` 26, `segment_padding` 9, `tab` 34, `tab_padding` 10, `chip` 22, `chip_padding` 9 | the kit's |

The application registers the files of its own faces with `typography::add_fonts`, then names
their families in the look. The mono face (Geist Mono: paths, counts, times) is not part of it.
`look()` gives the look the kit draws with, its sizes on the grid.

A pixel look: notched corners, a 2 px border, a hard shadow, a 4 px grid and faces of its own:

```rust
use std::borrow::Cow;

use gpui_design_system::look::Role;
use gpui_design_system::{Corners, Fonts, Icons, Identity, Look, Shadow, Sizes, typography};
use gpui_kit::px;

fn pixel() -> Look {
    Look {
        fonts: Fonts {
            ui: "Pixel Sans".into(),
            title: Some("Pixel Display".into()),
            control: Some("Pixel Mono".into()),
        },
        corners: Corners::Notch { step: px(2.), steps: 1 },
        border: px(2.),
        // The stronger border colour: black would hardly read on a dark page.
        shadow: Some(Shadow { x: px(4.), y: px(4.), color: Role::Line2 }),
        grid: Some(px(4.)),
        sizes: Sizes { button: px(32.), ..Sizes::default() },
    }
}

gpui_kit::application().with_assets(Icons).run(|cx| {
    typography::add_fonts(
        [
            Cow::Borrowed(include_bytes!("../fonts/PixelSans.ttf").as_slice()),
            Cow::Borrowed(include_bytes!("../fonts/PixelDisplay.ttf").as_slice()),
            Cow::Borrowed(include_bytes!("../fonts/PixelMono.ttf").as_slice()),
        ],
        cx,
    );
    gpui_design_system::init_with(Identity::new("Notes"), pixel(), cx);
    // Open the windows.
});
```

GPUI draws no cut corner: a notched surface is drawn by layers under its children, its border
keeping its room. A child that overflows a notched card or list (a row scrolled under its edge)
can cover the border and the cut corners, where a rounded one is clipped inside its border.

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

## Chips that can be removed, and a picker of them

`RemovableChip::new(id, label)` is a tag that can be taken away: its `.detail(..)` in the faint
ink, a remove button (the kit's icon button, tooltip "Remove <label>") that calls
`.on_remove(..)`. It unfolds sideways and fades in when it is first drawn, folds away when
`.present(false)` (build it on every frame while it folds), and fades its hover; `.at_rest()`
draws it whole from its first frame. It is not `Chip` (a read-only fact under a title) nor `Tag`.

`ChipPicker::new(id, label, &input, options, chosen)` is a multi-select whose value is an ordered
list of keys: the application keeps it and is told the new list by `.on_change(keys)`. `options`
are `ChipOption::new(key, label).detail(..)`; `input` is an `InputState` the caller made with
`follow_theme`, as for `SearchField`.

- The chosen keys are chips in the order added, then a search field; above them the label, a
  count once something is chosen, and "None" while nothing is.
- The field opens a menu of what is not chosen yet, filtered by `fuzzy` as the user types, each
  item with its detail. It opens below the field, above it when the window has no room, and over
  the dialog it may be in. It scrolls past 272 px, and stays open after an add.
- Keys: the arrows move the mark, Enter adds, Escape closes the menu (and nothing around it),
  Backspace in an empty field removes the last chip.
- `MenuItem::marked(bool)` is the row the keys are on: its fill fades over `HOVER`.

## A QR code

`QrCode::new(id, text, label)` draws the QR code of `text` (a link, a pairing code): near-black
modules on a near-white plate, with the quiet zone of 4 modules inside the plate. The colours are
the same in every theme, dark ones included, on purpose: a scanner expects dark on light, and a
plate that followed the palette would pass through mid-greys while a theme changes, at a moment
it does not scan.

- `label` names what the code holds for assistive technology (never the text itself).
- `.size(px)` is the most it takes a side (288 by default). The module is the whole number of
  pixels that fits, at least 1 px, so the plate can be a little smaller than `size`, and larger
  under (modules + 8) px.
- `.shown(false)` folds it away and fades it out; `true` unfolds it and fades it in.
- The error correction is Medium, which keeps a text of 180 characters (a link and a base32 code)
  at 53 modules. A text over 2331 bytes draws nothing at all: no plate, no label, no unfolding.
- The encoder is the `qrcodegen` crate (MIT).

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
