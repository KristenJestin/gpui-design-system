//! [`MachineMark`]: a machine's own icon in its own colour, with a small badge on its bottom-right
//! corner that says its state by shape, never by the machine's colour:
//!
//! - [`Here`](MachineState::Here): a house;
//! - [`Online`](MachineState::Online): a filled dot;
//! - [`Offline`](MachineState::Offline): a hollow ring, the icon greyed;
//! - [`Update`](MachineState::Update): a dot with an up arrow;
//! - [`Unknown`](MachineState::Unknown): a dashed ring, the icon a little faded.
//!
//! The badge hangs out of the icon's box (by 0.24 of its side, right and bottom): leave room for
//! it. It sits on a cut-out of the surface colour behind it, so it reads on a hovered or chosen
//! row. A state that changes fades the old shape out and the new one in, scaling, and moves the
//! icon's colour, over [`PANEL`]; with reduced motion they jump.

use gpui_kit::{
    AnyElement, App, Bounds, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement,
    PathBuilder, Pixels, Point, RenderOnce, SharedString, StatefulInteractiveElement, Styled,
    TestSupportExt, Window, canvas, div, point, px, relative,
};

use crate::button::tipped;
use crate::icons::{Icon, icon};
use crate::ids::part;
use crate::interact::{fade, tint};
use crate::motion::{HOVER, PANEL, SWAP_FROM};
use crate::theme::{Palette, palette};

/// What a machine is doing, said by the shape of the badge on its mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MachineState {
    /// The machine this is running on: a house.
    Here,
    /// Reachable: a filled dot.
    Online,
    /// Switched off or gone: a hollow ring, the icon greyed.
    Offline,
    /// Reachable but out of date: a dot with an up arrow.
    Update,
    /// Not reachable, or not tried yet: a dashed ring, the icon a little faded.
    Unknown,
}

impl MachineState {
    const ALL: [Self; 5] = [
        Self::Here,
        Self::Online,
        Self::Offline,
        Self::Update,
        Self::Unknown,
    ];

    /// The name of the shape's part: a test finds it as `<id>-<name>`.
    fn name(self) -> &'static str {
        match self {
            Self::Here => "here",
            Self::Online => "online",
            Self::Offline => "offline",
            Self::Update => "update",
            Self::Unknown => "unknown",
        }
    }

    /// The colour of the shape, from the palette alone.
    fn shape_colour(self, p: &Palette) -> Hsla {
        match self {
            Self::Here => p.text,
            Self::Online => p.ok,
            Self::Offline | Self::Unknown => p.faint,
            Self::Update => p.wait,
        }
    }

    /// The colour of the icon: the machine's, greyed when it is off, a little faded when not known.
    fn ink(self, machine: Hsla, p: &Palette) -> Hsla {
        match self {
            Self::Offline => p.faint,
            Self::Unknown => Hsla {
                a: machine.a * UNKNOWN_SHARE,
                ..machine
            },
            Self::Here | Self::Online | Self::Update => machine,
        }
    }
}

/// The three sizes of a mark, by the side of its icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkSize {
    /// 16 px: a heading.
    Large,
    /// 14 px: a mark in a row.
    Medium,
    /// 12 px: a mark in a line of small text.
    Small,
}

impl MarkSize {
    fn side(self) -> f32 {
        match self {
            Self::Large => 16.,
            Self::Medium => 14.,
            Self::Small => 12.,
        }
    }
}

/// How much of the machine's colour an icon of a machine not known keeps.
const UNKNOWN_SHARE: f32 = 0.7;
/// The badge's side, as a share of the icon's.
const BADGE: f32 = 0.58;
/// How far the badge hangs out of the icon's box, right and bottom, as a share of its side.
const HANG: f32 = 0.24;
/// How far the cut-out reaches beyond the badge, on every side.
const CUT: f32 = 1.4;

/// A machine's mark: `MachineMark::new(id, icon, colour, state, label)`. The icon and its colour
/// are the application's (a machine keeps its own whatever its state); `label` says it in words
/// (`server · offline since 09:33`) and is the mark's tooltip and accessible label. The element
/// is `<id>`, its icon `<id>-icon`, its badge `<id>-badge`, the cut-out under it `<id>-cutout`,
/// and the shape drawn `<id>-here`, `-online`, `-offline`, `-update` or `-unknown`.
///
/// Build it on every frame: it is its own state, and a first state is simply there.
#[derive(IntoElement)]
pub struct MachineMark {
    id: ElementId,
    icon: Icon,
    colour: Hsla,
    state: MachineState,
    label: SharedString,
    size: MarkSize,
    surface: Option<Hsla>,
}

impl MachineMark {
    pub fn new(
        id: impl Into<ElementId>,
        icon: Icon,
        colour: Hsla,
        state: MachineState,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            icon,
            colour,
            state,
            label: label.into(),
            size: MarkSize::Large,
            surface: None,
        }
    }

    pub fn size(mut self, size: MarkSize) -> Self {
        self.size = size;
        self
    }

    /// The colour of what the mark sits on, which the badge's cut-out takes: the left tree's
    /// unless said. A row that is hovered or chosen says its own, and the cut-out follows it over
    /// [`HOVER`].
    pub fn on(mut self, surface: Hsla) -> Self {
        self.surface = Some(surface);
        self
    }
}

impl RenderOnce for MachineMark {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let side = self.size.side();
        let badge = side * BADGE;

        let ink = tint(
            part(&self.id, "icon"),
            self.state.ink(self.colour, &p),
            PANEL,
            window,
            cx,
        );
        crate::seen::note_colour(&part(&self.id, "icon"), ink);
        let cutout = tint(
            part(&self.id, "cutout"),
            self.surface.unwrap_or(p.side),
            HOVER,
            window,
            cx,
        );
        crate::seen::note_colour(&part(&self.id, "cutout"), cutout);

        let mut shapes = Vec::new();
        for state in MachineState::ALL {
            let shape = part(&self.id, state.name());
            let t = fade(shape.clone(), state == self.state, PANEL, window, cx);
            // A shape that is gone and at rest is not drawn at all.
            if t <= 0.0 {
                continue;
            }
            crate::seen::note(&shape, t);
            crate::seen::note_colour(&shape, state.shape_colour(&p));
            shapes.push(shape_frame(shape, state, badge, t, &p));
        }

        let badge = div()
            .id(part(&self.id, "badge"))
            .absolute()
            .right(px(-side * HANG))
            .bottom(px(-side * HANG))
            .size(px(badge))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .id(part(&self.id, "cutout"))
                    .absolute()
                    .top(px(-CUT))
                    .left(px(-CUT))
                    .size(px(badge + 2. * CUT))
                    .rounded_full()
                    .bg(cutout)
                    .test_support(),
            )
            .children(shapes)
            .test_support();

        let mark = div()
            .id(self.id.clone())
            .aria_label(self.label.clone())
            .relative()
            .flex_none()
            .size(px(side))
            .child(
                div()
                    .id(part(&self.id, "icon"))
                    .size_full()
                    .child(icon(self.icon).size(px(side)).text_color(ink)),
            )
            .child(badge);
        tipped(mark, self.label, side, window, cx)
    }
}

/// One shape of the badge, `t` of the way in (0 gone, 1 there): a square of the badge's side,
/// scaled from [`SWAP_FROM`] and faded with it. The house, the arrow and the dashes are painted as
/// paths in a box of ten units, since GPUI draws neither a filled house nor a dashed circle from a
/// style.
fn shape_frame(id: ElementId, state: MachineState, badge: f32, t: f32, p: &Palette) -> AnyElement {
    let scale = SWAP_FROM + (1.0 - SWAP_FROM) * t;
    let colour = state.shape_colour(p);
    let ink = p.on_wait;
    let frame = div()
        .id(id)
        .absolute()
        .size(px(badge * scale))
        .opacity(t)
        .flex()
        .items_center()
        .justify_center();
    match state {
        MachineState::Online => frame.child(div().size(relative(0.88)).rounded_full().bg(colour)),
        MachineState::Offline => frame.child(
            div()
                .size(relative(0.8))
                .rounded_full()
                .border(px(badge * scale * 0.15))
                .border_color(colour),
        ),
        MachineState::Update => frame.child(div().size_full().rounded_full().bg(colour).child(
            painted(move |at, unit, window| paint_arrow(at, unit, ink, window)),
        )),
        MachineState::Here => frame.child(painted(move |at, unit, window| {
            paint_house(at, unit, colour, window)
        })),
        MachineState::Unknown => frame.child(painted(move |at, unit, window| {
            paint_dashes(at, unit, colour, window)
        })),
    }
    .test_support()
    .into_any_element()
}

/// A canvas over the whole shape that paints with `draw(origin, unit, window)`, the unit being a
/// tenth of the shape's side.
fn painted(draw: impl Fn(Point<Pixels>, f32, &mut Window) + 'static) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds: Bounds<Pixels>, (), window, _| {
            draw(bounds.origin, f32::from(bounds.size.width) / 10.0, window)
        },
    )
    .size_full()
}

fn at(origin: Point<Pixels>, unit: f32, x: f32, y: f32) -> Point<Pixels> {
    origin + point(px(x * unit), px(y * unit))
}

/// A solid house: a roof over a body, filling the shape.
fn paint_house(origin: Point<Pixels>, unit: f32, colour: Hsla, window: &mut Window) {
    let mut house = PathBuilder::fill();
    house.add_polygon(
        &[
            at(origin, unit, 5.0, 0.6),
            at(origin, unit, 9.4, 4.3),
            at(origin, unit, 9.4, 9.4),
            at(origin, unit, 0.6, 9.4),
            at(origin, unit, 0.6, 4.3),
        ],
        true,
    );
    if let Ok(path) = house.build() {
        window.paint_path(path, colour);
    }
}

/// An arrow pointing up, in the middle of the shape.
fn paint_arrow(origin: Point<Pixels>, unit: f32, colour: Hsla, window: &mut Window) {
    let mut arrow = PathBuilder::stroke(px(1.3 * unit));
    arrow.move_to(at(origin, unit, 5.0, 7.5));
    arrow.line_to(at(origin, unit, 5.0, 2.9));
    arrow.move_to(at(origin, unit, 2.9, 4.9));
    arrow.line_to(at(origin, unit, 5.0, 2.8));
    arrow.line_to(at(origin, unit, 7.1, 4.9));
    if let Ok(path) = arrow.build() {
        window.paint_path(path, colour);
    }
}

/// A ring of dashes: a circle of 3.5 units as a polygon, stroked 1.5 units wide with a dash of
/// 1.5 and a gap of 1.7.
fn paint_dashes(origin: Point<Pixels>, unit: f32, colour: Hsla, window: &mut Window) {
    const POINTS: usize = 48;
    let ring: Vec<_> = (0..POINTS)
        .map(|i| {
            let turn = std::f32::consts::TAU * i as f32 / POINTS as f32;
            at(origin, unit, 5.0 + 3.5 * turn.cos(), 5.0 + 3.5 * turn.sin())
        })
        .collect();
    let mut dashes =
        PathBuilder::stroke(px(1.5 * unit)).dash_array(&[px(1.5 * unit), px(1.7 * unit)]);
    dashes.add_polygon(&ring, true);
    if let Ok(path) = dashes.build() {
        window.paint_path(path, colour);
    }
}
