//! The look: the faces, the corners, the borders, the shadow and the sizes an application gives
//! the kit once, at [`init_with`](crate::init_with). Every field defaults to the kit's own design,
//! so an application that gives none is drawn exactly as before. Colours are not part of it: they
//! stay the theme's ([`palette`](crate::palette)).
//!
//! The look is kept for the thread that runs the application (GPUI draws on one thread): the
//! faces are applied by [`Type`](crate::Type), whose methods are on any styled element and have
//! no context to read a global from, and a component reads the same value where it draws. Each
//! test of GPUI's test platform runs on a thread of its own, with its own look.

use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, BoxShadow, Hsla, IntoElement, ParentElement, Pixels, SharedString, Styled, div,
    point, px, transparent_black,
};

use crate::theme::Palette;
use crate::typography::UI;

/// What an application restyles of the kit. `Look::default()` is the kit's own design.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub fonts: Fonts,
    pub corners: Corners,
    /// The width of every border and separating line: 1 px.
    pub border: Pixels,
    /// A hard shadow under the controls and the overlays, which a pressed button moves onto. None:
    /// the kit's soft shadows, and a pressed button shrinks a little.
    pub shadow: Option<Shadow>,
    /// When set, the look's sizes, paddings and shadow offset are whole multiples of this unit,
    /// rounded to the nearest (never below one unit). The border and a notch's step are a
    /// pixel art's own lines: they are kept as given.
    pub grid: Option<Pixels>,
    pub sizes: Sizes,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            fonts: Fonts::default(),
            corners: Corners::Rounded,
            border: px(1.),
            shadow: None,
            grid: None,
            sizes: Sizes::default(),
        }
    }
}

/// The faces, by family name. The application registers the files of its own faces
/// ([`add_fonts`](crate::typography::add_fonts)); the kit bundles Geist and Geist Mono. The mono
/// face (Geist Mono, for paths, counts and times) is not part of the look.
#[derive(Clone, Debug, PartialEq)]
pub struct Fonts {
    /// The interface: every text of the kit ([`Type::ui_text`](crate::Type::ui_text)). Geist.
    pub ui: SharedString,
    /// The titles of pages, sections, cards, dialogs and drawers. None: the interface face.
    pub title: Option<SharedString>,
    /// The words of the controls: buttons, segmented controls, selects, tabs, chips. None: the
    /// interface face.
    pub control: Option<SharedString>,
}

impl Default for Fonts {
    fn default() -> Self {
        Self {
            ui: SharedString::new_static(UI),
            title: None,
            control: None,
        }
    }
}

/// How the corners of a surface (a button, a card, a field, a menu) are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Corners {
    /// The kit's own radii ([`theme::radius`](crate::theme::radius) and each component's).
    #[default]
    Rounded,
    /// Square corners.
    Square,
    /// Corners cut by whole pixels: `steps` stairs of `step` px each way, the border following
    /// them. One step of 2 px cuts a 2 px square from each corner. A step is at least the border's
    /// width.
    Notch { step: Pixels, steps: u8 },
}

/// A hard drop shadow: the surface's own shape, `x` px right and `y` px down, with no blur, in
/// the palette's colour of the role `color`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub x: Pixels,
    pub y: Pixels,
    pub color: Role,
}

/// A role of the palette a hard shadow is drawn in: its colour comes from the theme shown, a
/// desktop's included.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Role {
    /// [`Palette::shadow`], at full strength: the overlays' own shadow colour.
    #[default]
    Shadow,
    /// [`Palette::line`]: the borders.
    Line,
    /// [`Palette::line2`]: the stronger borders, which read on a dark page too.
    Line2,
    /// [`Palette::text`]: the ink.
    Text,
    /// [`Palette::faint`]: the faintest ink.
    Faint,
    /// [`Palette::acc`]: the primary.
    Accent,
}

impl Role {
    /// The colour of this role in `palette`.
    pub fn of(self, palette: &Palette) -> Hsla {
        match self {
            Self::Shadow => palette.shadow,
            Self::Line => palette.line,
            Self::Line2 => palette.line2,
            Self::Text => palette.text,
            Self::Faint => palette.faint,
            Self::Accent => palette.acc,
        }
    }
}

/// The heights of the controls and the space on each side of their words.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sizes {
    /// A text button's height, and a round button's side: 30 px.
    pub button: Pixels,
    /// The space on each side of a text button's label: 12 px.
    pub button_padding: Pixels,
    /// An icon button's side: 28 px.
    pub icon_button: Pixels,
    /// A floating button's side: 32 px.
    pub float_button: Pixels,
    /// A select's field and a search field: 34 px.
    pub field: Pixels,
    /// The space before the words of a field: 10 px.
    pub field_padding: Pixels,
    /// A compact select's pill: 24 px.
    pub pill: Pixels,
    /// The space on each side of a pill's words: 9 px.
    pub pill_padding: Pixels,
    /// A segmented control: 26 px.
    pub segmented: Pixels,
    /// The space on each side of a segment's words: 9 px.
    pub segment_padding: Pixels,
    /// The row of tabs: 34 px.
    pub tab: Pixels,
    /// The space on each side of a tab's words: 10 px.
    pub tab_padding: Pixels,
    /// A filter chip: 22 px.
    pub chip: Pixels,
    /// The space on each side of a chip's words: 9 px.
    pub chip_padding: Pixels,
}

impl Default for Sizes {
    fn default() -> Self {
        Self {
            button: px(30.),
            button_padding: px(12.),
            icon_button: px(28.),
            float_button: px(32.),
            field: px(34.),
            field_padding: px(10.),
            pill: px(24.),
            pill_padding: px(9.),
            segmented: px(26.),
            segment_padding: px(9.),
            tab: px(34.),
            tab_padding: px(10.),
            chip: px(22.),
            chip_padding: px(9.),
        }
    }
}

impl Sizes {
    fn map(self, f: impl Fn(Pixels) -> Pixels) -> Self {
        Self {
            button: f(self.button),
            button_padding: f(self.button_padding),
            icon_button: f(self.icon_button),
            float_button: f(self.float_button),
            field: f(self.field),
            field_padding: f(self.field_padding),
            pill: f(self.pill),
            pill_padding: f(self.pill_padding),
            segmented: f(self.segmented),
            segment_padding: f(self.segment_padding),
            tab: f(self.tab),
            tab_padding: f(self.tab_padding),
            chip: f(self.chip),
            chip_padding: f(self.chip_padding),
        }
    }
}

impl Look {
    /// `value` on the grid: the nearest whole multiple of its unit, at least one unit for a value
    /// that is not zero, the sign kept. Without a grid, `value` itself.
    pub fn snap(&self, value: Pixels) -> Pixels {
        let Some(unit) = self.grid.map(f32::from).filter(|unit| *unit > 0.) else {
            return value;
        };
        let v = f32::from(value);
        if v == 0. {
            return value;
        }
        let steps = (v.abs() / unit).round().max(1.);
        px(v.signum() * steps * unit)
    }

    /// The look as the components draw it: its sizes snapped to the grid, a notch's step at least
    /// the border.
    pub fn resolved(&self) -> Self {
        let snap = |value| self.snap(value);
        let corners = match self.corners {
            Corners::Notch { step, steps } => Corners::Notch {
                step: step.max(self.border),
                steps: steps.max(1),
            },
            other => other,
        };
        Self {
            fonts: self.fonts.clone(),
            corners,
            border: self.border,
            shadow: self.shadow.map(|shadow| Shadow {
                x: snap(shadow.x),
                y: snap(shadow.y),
                color: shadow.color,
            }),
            grid: self.grid,
            sizes: self.sizes.map(snap),
        }
    }
}

thread_local! {
    static CURRENT: RefCell<Rc<Look>> = RefCell::new(Rc::new(Look::default()));
}

/// The look the kit draws with, as given to [`init_with`](crate::init_with) and resolved
/// ([`Look::resolved`]); the kit's own before.
pub fn look() -> Rc<Look> {
    CURRENT.with(|current| current.borrow().clone())
}

/// Makes `look` the one the kit draws with on this thread.
pub(crate) fn install(look: &Look) {
    let resolved = Rc::new(look.resolved());
    CURRENT.with(|current| *current.borrow_mut() = resolved);
}

/// A corner's radius in the kit's own design.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Radius {
    Px(Pixels),
    /// As round as the element is: a pill, a circle.
    Full,
}

impl From<Pixels> for Radius {
    fn from(radius: Pixels) -> Self {
        Self::Px(radius)
    }
}

/// Which corners are shaped; the others are square.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Which {
    #[default]
    All,
    Top,
    Bottom,
    BottomLeft,
}

/// A side of an element, for a separating line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

/// What a surface is drawn with: its corners in the kit's own design, its fill and its border.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Surface {
    radius: Radius,
    which: Which,
    fill: Option<Hsla>,
    line: Option<Hsla>,
    dashed: bool,
}

impl Surface {
    pub(crate) fn new(radius: impl Into<Radius>) -> Self {
        Self {
            radius: radius.into(),
            which: Which::All,
            fill: None,
            line: None,
            dashed: false,
        }
    }

    pub(crate) fn full() -> Self {
        Self::new(Radius::Full)
    }

    /// Only these corners are shaped.
    pub(crate) fn at(mut self, which: Which) -> Self {
        self.which = which;
        self
    }

    pub(crate) fn fill(self, fill: Hsla) -> Self {
        self.fill_some(Some(fill))
    }

    pub(crate) fn fill_some(mut self, fill: Option<Hsla>) -> Self {
        self.fill = fill;
        self
    }

    /// A border of the look's width all around, in `line`.
    pub(crate) fn line(self, line: Hsla) -> Self {
        self.line_some(Some(line))
    }

    pub(crate) fn line_some(mut self, line: Option<Hsla>) -> Self {
        self.line = line;
        self
    }

    pub(crate) fn dashed(mut self) -> Self {
        self.dashed = true;
        self
    }
}

/// The look applied to any element that holds children.
pub(crate) trait Shaped: Styled + ParentElement + Sized {
    /// Only the radii of the corners: the kit's own, or none in a square or notched look. For the
    /// small marks drawn inside a control (a knob, a dot, a tick's box) and for what is drawn
    /// with no surface of its own.
    fn corners(self, radius: impl Into<Radius>) -> Self {
        self.corners_at(radius, Which::All)
    }

    fn corners_at(mut self, radius: impl Into<Radius>, which: Which) -> Self {
        let r = match (look().corners, radius.into()) {
            (Corners::Rounded, Radius::Px(r)) => r,
            (Corners::Rounded, Radius::Full) if which == Which::All => return self.rounded_full(),
            (Corners::Rounded, Radius::Full) => px(9999.),
            _ => px(0.),
        };
        if which == Which::All {
            return self.rounded(r);
        }
        let corners = &mut self.style().corner_radii;
        let r = Some(r.into());
        match which {
            Which::All => {}
            Which::Top => {
                corners.top_left = r;
                corners.top_right = r;
            }
            Which::Bottom => {
                corners.bottom_left = r;
                corners.bottom_right = r;
            }
            Which::BottomLeft => corners.bottom_left = r,
        }
        self
    }

    /// A separating line of the look's width on `side`, in `color`.
    fn edge(mut self, side: Side, color: Hsla) -> Self {
        let width = Some(look().border.into());
        let widths = &mut self.style().border_widths;
        match side {
            Side::Top => widths.top = width,
            Side::Bottom => widths.bottom = width,
            Side::Left => widths.left = width,
            Side::Right => widths.right = width,
        }
        self.border_color(color)
    }

    /// The surface drawn in the look. Called before the element's children are given: in a
    /// notched look, the surface is drawn by layers under them.
    fn surface(self, surface: Surface) -> Self {
        let look = look();
        let Corners::Notch { step, steps } = look.corners else {
            let shaped = self.corners_at(surface.radius, surface.which);
            let shaped = match surface.fill {
                Some(fill) => shaped.bg(fill),
                None => shaped,
            };
            return match surface.line {
                Some(line) if surface.dashed => shaped
                    .border(look.border)
                    .border_color(line)
                    .border_dashed(),
                Some(line) => shaped.border(look.border).border_color(line),
                None => shaped,
            };
        };
        let (top, bottom) = match surface.which {
            Which::All => (steps, steps),
            Which::Top => (steps, 0),
            Which::Bottom => (0, steps),
            Which::BottomLeft => (0, 0),
        };
        let border = surface.line.map_or(0., |_| f32::from(look.border));
        let notch = Notch {
            step: f32::from(step),
            top,
            bottom,
            border,
        };
        // The border keeps its room, unpainted: the layers draw it along the notch.
        let shaped = match surface.line {
            Some(_) => self.border(look.border).border_color(transparent_black()),
            None => self,
        };
        shaped.children(notch.layers(surface.fill, surface.line, surface.dashed))
    }

    /// GPUI's small soft shadow, under a floating button; none in a look with a hard shadow, which
    /// the button draws itself.
    fn float_shadow(self) -> Self {
        match look().shadow {
            Some(_) => self,
            None => self.shadow_sm(),
        }
    }

    /// A shadow: the look's hard one, in its role of `palette`, when it has one; `soft`
    /// otherwise.
    fn drop_shadow(self, soft: Vec<BoxShadow>, palette: &Palette) -> Self {
        match look().shadow {
            Some(shadow) => self.shadow(vec![hard_shadow(shadow, palette)]),
            None => self.shadow(soft),
        }
    }
}

impl<T: Styled + ParentElement> Shaped for T {}

/// The look's hard shadow as GPUI draws one: no blur, no spread.
pub(crate) fn hard_shadow(shadow: Shadow, palette: &Palette) -> BoxShadow {
    BoxShadow {
        color: shadow.color.of(palette),
        offset: point(shadow.x, shadow.y),
        blur_radius: px(0.),
        spread_radius: px(0.),
        inset: false,
    }
}

/// A notched surface, as layers placed by their insets in the element: `top` and `bottom` stairs
/// of `step` px at the top and bottom corners, and a border `border` px wide (0 for none).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Notch {
    pub step: f32,
    pub top: u8,
    pub bottom: u8,
    pub border: f32,
}

/// One layer of a notched surface: a rectangle given by its insets in the element and its size,
/// filled or a piece of the border.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Piece {
    pub top: Option<f32>,
    pub bottom: Option<f32>,
    pub left: Option<f32>,
    pub right: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    /// A piece of the border, and which side of the surface it runs along.
    pub line: Option<Side>,
}

/// A horizontal band of a notched surface: inset `inset` px on both sides, from `top` (or from
/// `bottom`) for `height`, or between `top` and `bottom`.
struct Band {
    inset: f32,
    top: Option<f32>,
    bottom: Option<f32>,
    height: Option<f32>,
}

impl Notch {
    /// The bands of the surface, from the top down: the top stairs, the middle, the bottom stairs.
    fn bands(&self) -> Vec<Band> {
        let s = self.step;
        let (top, bottom) = (f32::from(self.top), f32::from(self.bottom));
        let mut bands: Vec<Band> = (0..self.top)
            .map(|i| Band {
                inset: (top - f32::from(i)) * s,
                top: Some(f32::from(i) * s),
                bottom: None,
                height: Some(s),
            })
            .collect();
        bands.push(Band {
            inset: 0.,
            top: Some(top * s),
            bottom: Some(bottom * s),
            height: None,
        });
        bands.extend((0..self.bottom).rev().map(|i| Band {
            inset: (bottom - f32::from(i)) * s,
            top: None,
            bottom: Some(f32::from(i) * s),
            height: Some(s),
        }));
        bands
    }

    /// The fill, then the border: pieces that never overlap, so a colour that is not opaque, or
    /// an element faded, is drawn evenly.
    pub(crate) fn pieces(&self) -> (Vec<Piece>, Vec<Piece>) {
        let bands = self.bands();
        let fill = bands
            .iter()
            .map(|band| Piece {
                top: band.top,
                bottom: band.bottom,
                left: Some(band.inset),
                right: Some(band.inset),
                height: band.height,
                ..Piece::default()
            })
            .collect();
        let b = self.border;
        if b <= 0. {
            return (fill, Vec::new());
        }
        let mut lines = Vec::new();
        // The two sides of each band.
        for band in &bands {
            for side in [Side::Left, Side::Right] {
                let (left, right) = match side {
                    Side::Left => (Some(band.inset), None),
                    _ => (None, Some(band.inset)),
                };
                lines.push(Piece {
                    top: band.top,
                    bottom: band.bottom,
                    left,
                    right,
                    width: Some(b),
                    height: band.height,
                    line: Some(side),
                });
            }
        }
        // The top and bottom edges, between the sides.
        let (first, last) = (&bands[0], &bands[bands.len() - 1]);
        lines.push(Piece {
            top: Some(0.),
            left: Some(first.inset + b),
            right: Some(first.inset + b),
            height: Some(b),
            line: Some(Side::Top),
            ..Piece::default()
        });
        lines.push(Piece {
            bottom: Some(0.),
            left: Some(last.inset + b),
            right: Some(last.inset + b),
            height: Some(b),
            line: Some(Side::Bottom),
            ..Piece::default()
        });
        // Each stair's tread, from the band above's side to the band below's (going down at the
        // top, up at the bottom).
        for pair in bands.windows(2) {
            let (above, below) = (&pair[0], &pair[1]);
            let (narrow, wide, top, bottom, side) = if above.inset > below.inset {
                (above, below, below.top, None, Side::Top)
            } else if below.inset > above.inset {
                (below, above, None, above.bottom, Side::Bottom)
            } else {
                continue;
            };
            for left_side in [true, false] {
                let inset = Some(wide.inset + b);
                let (left, right) = if left_side {
                    (inset, None)
                } else {
                    (None, inset)
                };
                lines.push(Piece {
                    top,
                    bottom,
                    left,
                    right,
                    width: Some(narrow.inset - wide.inset),
                    height: Some(b),
                    line: Some(side),
                });
            }
        }
        (fill, lines)
    }

    /// The layers: the fill in `fill`, then the border in `line`, placed over the element's
    /// border box (its border, unpainted, is outside the box its children are placed in).
    fn layers(&self, fill: Option<Hsla>, line: Option<Hsla>, dashed: bool) -> Vec<AnyElement> {
        let (fills, lines) = self.pieces();
        let shift = -self.border;
        let fills = fill
            .into_iter()
            .flat_map(|color| fills.iter().map(move |piece| place(piece, shift).bg(color)));
        let lines = line.into_iter().flat_map(|color| {
            let border = self.border;
            lines.iter().map(move |piece| {
                let layer = place(piece, shift);
                if !dashed {
                    return layer.bg(color);
                }
                let width = Some(px(border).into());
                let mut layer = layer.border_dashed().border_color(color);
                let widths = &mut layer.style().border_widths;
                match piece.line {
                    Some(Side::Left) => widths.left = width,
                    Some(Side::Right) => widths.right = width,
                    Some(Side::Bottom) => widths.bottom = width,
                    _ => widths.top = width,
                }
                layer
            })
        });
        fills.chain(lines).enumerate().map(observed).collect()
    }
}

/// A layer as drawn; the kit's own tests find it by its place among the layers.
fn observed((at, layer): (usize, gpui_kit::Div)) -> AnyElement {
    #[cfg(test)]
    {
        use gpui_kit::{InteractiveElement as _, TestSupportExt as _};
        layer.id(("notch", at)).test_support().into_any_element()
    }
    #[cfg(not(test))]
    {
        let _ = at;
        layer.into_any_element()
    }
}

/// An absolute layer at `piece`'s place, its insets moved by `shift`.
fn place(piece: &Piece, shift: f32) -> gpui_kit::Div {
    let at = |inset: Option<f32>| inset.map(|inset| px(inset + shift));
    div()
        .absolute()
        .when_some(at(piece.top), |d, v| d.top(v))
        .when_some(at(piece.bottom), |d, v| d.bottom(v))
        .when_some(at(piece.left), |d, v| d.left(v))
        .when_some(at(piece.right), |d, v| d.right(v))
        .when_some(piece.width, |d, v| d.w(px(v)))
        .when_some(piece.height, |d, v| d.h(px(v)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        Context, InteractiveElement as _, Render, TestAppContext, TestSupportExt as _, Window,
        hsla, size,
    };

    fn style(mut element: gpui_kit::Div) -> gpui_kit::StyleRefinement {
        element.style().clone()
    }

    #[test]
    fn in_the_kit_s_own_look_a_surface_has_the_styles_it_had_before() {
        install(&Look::default());
        let (fill, line) = (hsla(0.1, 0.2, 0.3, 1.), hsla(0.4, 0.5, 0.6, 0.7));
        let same = |now: gpui_kit::Div, before: gpui_kit::Div, what: &str| {
            assert_eq!(style(now), style(before), "{what}");
        };
        same(
            div().surface(Surface::new(px(7.)).fill(fill).line(line)),
            div().rounded(px(7.)).bg(fill).border_1().border_color(line),
            "a bordered surface",
        );
        same(
            div().surface(Surface::full().fill(fill)),
            div().rounded_full().bg(fill),
            "a pill",
        );
        same(
            div().surface(Surface::new(px(12.)).line(line).dashed()),
            div()
                .rounded(px(12.))
                .border_1()
                .border_dashed()
                .border_color(line),
            "a dashed outline",
        );
        same(
            div().surface(Surface::new(px(6.)).at(Which::Top).fill(fill)),
            div().rounded_t(px(6.)).bg(fill),
            "the top corners",
        );
        same(
            div().corners_at(px(4.), Which::BottomLeft),
            div().rounded_bl(px(4.)),
            "one corner",
        );
        same(div().corners(Radius::Full), div().rounded_full(), "a dot");
        same(
            div().edge(Side::Bottom, line),
            div().border_b_1().border_color(line),
            "a separating line",
        );
        same(
            div().edge(Side::Left, line),
            div().border_l_1().border_color(line),
            "a guide",
        );
        same(div().float_shadow(), div().shadow_sm(), "a floating button");
        let soft = vec![BoxShadow {
            color: line,
            offset: point(px(0.), px(8.)),
            blur_radius: px(24.),
            spread_radius: px(0.),
            inset: false,
        }];
        same(
            div().drop_shadow(soft.clone(), &Palette::base()),
            div().shadow(soft),
            "an overlay's shadow",
        );
        assert_eq!(Role::default(), Role::Shadow);
        assert_eq!(Role::Shadow.of(&Palette::base()), Palette::base().shadow);
    }

    #[test]
    fn a_hard_shadow_with_a_chosen_role_draws_in_that_role_s_colour() {
        type Of = fn(&Palette) -> Hsla;
        let roles: [(Role, Of); 6] = [
            (Role::Shadow, |p: &Palette| p.shadow),
            (Role::Line, |p: &Palette| p.line),
            (Role::Line2, |p: &Palette| p.line2),
            (Role::Text, |p: &Palette| p.text),
            (Role::Faint, |p: &Palette| p.faint),
            (Role::Accent, |p: &Palette| p.acc),
        ];
        for (role, of) in roles {
            install(&Look {
                shadow: Some(Shadow {
                    x: px(4.),
                    y: px(4.),
                    color: role,
                }),
                ..Look::default()
            });
            for p in [Palette::base(), Palette::light()] {
                let hard = vec![BoxShadow {
                    color: of(&p),
                    offset: point(px(4.), px(4.)),
                    blur_radius: px(0.),
                    spread_radius: px(0.),
                    inset: false,
                }];
                assert_eq!(
                    style(div().drop_shadow(Vec::new(), &p)),
                    style(div().shadow(hard)),
                    "{role:?}"
                );
            }
        }
        install(&Look::default());
    }

    #[test]
    fn in_a_square_look_a_surface_has_no_radius_and_the_look_s_border() {
        install(&Look {
            corners: Corners::Square,
            border: px(2.),
            shadow: Some(Shadow {
                x: px(3.),
                y: px(3.),
                color: Role::Line2,
            }),
            ..Look::default()
        });
        let line = hsla(0.4, 0.5, 0.6, 1.);
        let p = Palette::base();
        assert_eq!(
            style(div().surface(Surface::full().line(line))),
            style(div().rounded(px(0.)).border(px(2.)).border_color(line))
        );
        assert_eq!(
            style(div().drop_shadow(Vec::new(), &p)),
            style(div().shadow(vec![BoxShadow {
                color: p.line2,
                offset: point(px(3.), px(3.)),
                blur_radius: px(0.),
                spread_radius: px(0.),
                inset: false,
            }])),
            "a hard shadow"
        );
        install(&Look::default());
    }

    struct Notched;

    impl Render for Notched {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().p(px(10.)).child(
                div()
                    .id("box")
                    .w(px(40.))
                    .h(px(20.))
                    .p(px(3.))
                    .surface(
                        Surface::new(px(8.))
                            .fill(hsla(0.6, 0.2, 0.5, 1.))
                            .line(hsla(0.6, 0.2, 0.9, 1.)),
                    )
                    .child(div().id("inside").size_full().test_support())
                    .test_support(),
            )
        }
    }

    #[gpui_kit::test]
    fn a_notched_surface_is_drawn_on_the_element_s_whole_box(cx: &mut TestAppContext) {
        install(&Look {
            corners: Corners::Notch {
                step: px(2.),
                steps: 1,
            },
            border: px(2.),
            ..Look::default()
        });
        let (_, cx) = cx.add_window_view(|_, _| Notched);
        cx.simulate_resize(size(px(200.), px(100.)));
        cx.update(|window, cx| window.render_frame(cx));
        let found = |id: gpui_kit::ElementId, cx: &mut gpui_kit::VisualTestContext| {
            cx.update(|window, _| gpui_kit::test::TestWindowExt::find(window, id).bounds())
        };
        let frame = found("box".into(), cx);
        assert_eq!(
            frame.size,
            size(px(40.), px(20.)),
            "the border keeps its room"
        );
        let inside = found("inside".into(), cx);
        assert_eq!(
            inside.origin.x - frame.origin.x,
            px(5.),
            "border and padding"
        );
        // The first layer is the top band of the fill: the corners' 2 px cut on each side.
        let band = found(("notch", 0usize).into(), cx);
        assert_eq!(band.origin.x - frame.origin.x, px(2.));
        assert_eq!(band.origin.y, frame.origin.y);
        assert_eq!(band.size, size(px(36.), px(2.)));
        // The middle band spans the whole width.
        let middle = found(("notch", 1usize).into(), cx);
        assert_eq!(middle.origin.x, frame.origin.x);
        assert_eq!(middle.size, size(px(40.), px(16.)));
        // The first piece of the border: the top band's left side.
        let side = found(("notch", 3usize).into(), cx);
        assert_eq!(side.origin.x - frame.origin.x, px(2.));
        assert_eq!(side.origin.y, frame.origin.y);
        assert_eq!(side.size, size(px(2.), px(2.)));
        install(&Look::default());
    }

    fn notch(steps: u8, border: f32) -> Notch {
        Notch {
            step: 2.,
            top: steps,
            bottom: steps,
            border,
        }
    }

    /// Paints the pieces on a `w` by `h` grid of pixels, counting how many cover each.
    fn coverage(pieces: &[Piece], w: usize, h: usize) -> Vec<Vec<u8>> {
        let mut grid = vec![vec![0u8; w]; h];
        for piece in pieces {
            let (wf, hf) = (w as f32, h as f32);
            let (x0, x1) = match (piece.left, piece.right, piece.width) {
                (Some(l), Some(r), _) => (l, wf - r),
                (Some(l), None, Some(width)) => (l, l + width),
                (None, Some(r), Some(width)) => (wf - r - width, wf - r),
                other => panic!("a piece placed sideways: {other:?}"),
            };
            let (y0, y1) = match (piece.top, piece.bottom, piece.height) {
                (Some(t), Some(b), _) => (t, hf - b),
                (Some(t), None, Some(height)) => (t, t + height),
                (None, Some(b), Some(height)) => (hf - b - height, hf - b),
                other => panic!("a piece placed up and down: {other:?}"),
            };
            for row in grid.iter_mut().take(y1 as usize).skip(y0 as usize) {
                for cell in row.iter_mut().take(x1 as usize).skip(x0 as usize) {
                    *cell += 1;
                }
            }
        }
        grid
    }

    #[test]
    fn a_notch_fills_its_shape_once_with_its_corners_cut() {
        let (fill, _) = notch(2, 0.).pieces();
        let grid = coverage(&fill, 12, 10);
        // Two stairs of 2 px: the corner's 4 px square loses its outer stair and a half.
        for (y, row) in grid.iter().enumerate() {
            for (x, &cell) in row.iter().enumerate() {
                let (dx, dy) = (x.min(11 - x), y.min(9 - y));
                let cut = (dy < 2 && dx < 4) || (dy < 4 && dx < 2);
                assert_eq!(cell, u8::from(!cut), "pixel {x},{y}");
            }
        }
    }

    #[test]
    fn a_notch_s_border_follows_the_stairs_and_never_covers_a_pixel_twice() {
        let (fill, lines) = notch(1, 2.).pieces();
        let filled = coverage(&fill, 10, 10);
        let lined = coverage(&lines, 10, 10);
        for y in 0..10 {
            for x in 0..10 {
                assert!(lined[y][x] <= 1, "pixel {x},{y} is drawn twice");
                if lined[y][x] == 1 {
                    assert_eq!(filled[y][x], 1, "the border at {x},{y} is on the shape");
                }
            }
        }
        // Along the top: the corner cut, then the border, then the fill.
        assert_eq!(lined[0][..3], [0, 0, 1]);
        assert_eq!(lined[2][..3], [1, 1, 1], "the tread of the stair");
        assert_eq!(lined[4][..3], [1, 1, 0], "the side, under the stair");
        assert_eq!(filled[4][2], 1);
        assert_eq!(lined[6][..3], [1, 1, 1], "the bottom stair's tread");
        assert_eq!(lined[9][..3], [0, 0, 1], "the bottom corner cut");
        assert_eq!(lined[9][3..7], [1, 1, 1, 1], "the bottom edge");
        assert_eq!(lined[0][3..7], [1, 1, 1, 1], "the top edge");
    }

    #[test]
    fn a_grid_snaps_every_size_of_the_look_to_its_unit() {
        let look = Look {
            grid: Some(px(4.)),
            shadow: Some(Shadow {
                x: px(3.),
                y: px(-1.),
                color: Role::Shadow,
            }),
            corners: Corners::Notch {
                step: px(1.),
                steps: 1,
            },
            ..Look::default()
        }
        .resolved();
        assert_eq!(look.sizes.button, px(32.));
        assert_eq!(look.sizes.icon_button, px(28.));
        assert_eq!(look.sizes.chip, px(24.));
        assert_eq!(look.sizes.segment_padding, px(8.));
        assert_eq!(
            look.shadow,
            Some(Shadow {
                x: px(4.),
                y: px(-4.),
                color: Role::Shadow,
            }),
            "an offset is at least one unit, its sign kept"
        );
        assert_eq!(look.border, px(1.), "a border is a line, not a size");
        assert_eq!(
            look.corners,
            Corners::Notch {
                step: px(1.),
                steps: 1
            }
        );
        let thick = Look {
            border: px(3.),
            ..look
        }
        .resolved();
        assert_eq!(
            thick.corners,
            Corners::Notch {
                step: px(3.),
                steps: 1
            },
            "a step at least the border"
        );
    }
}
