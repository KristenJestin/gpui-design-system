//! The look an application gives the kit (#45): its faces, corners, borders, shadow and sizes,
//! each defaulting to the kit's own design.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{advance, init, init_look};
use gpui_design_system::look::Role;
use gpui_design_system::motion::PRESS;
use gpui_design_system::{
    Button, Corners, Dropdown, FilterChip, FilterChips, Fonts, Icon, Look, Segmented, Select,
    SelectOption, Shadow, Sizes, Tabs, Type, look,
};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AnyElement, Bounds, Context, Entity, IntoElement, Modifiers, MouseButton, ParentElement,
    Pixels, Render, SharedString, Styled, TestAppContext, VisualTestContext, Window, div, px, size,
};

type Draw = Box<dyn Fn(&mut Window, &mut Context<Host>) -> AnyElement>;

struct Host {
    draw: Draw,
}

impl Render for Host {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().relative().child((self.draw)(window, cx))
    }
}

fn host(
    cx: &mut TestAppContext,
    draw: impl Fn(&mut Window, &mut Context<Host>) -> AnyElement + 'static,
) -> (Entity<Host>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(move |_, _| Host {
        draw: Box::new(draw),
    });
    cx.simulate_resize(size(px(1000.), px(800.)));
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

fn bounds(id: &str, cx: &mut VisualTestContext) -> Bounds<Pixels> {
    let id = id.to_owned();
    cx.update(|window, _| window.find(id).bounds())
}

/// Every control whose height the look gives, side by side.
fn controls(_: &mut Window, _: &mut Context<Host>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .items_start()
        .gap(px(8.))
        .child(Button::primary("primary", "Save"))
        .child(Button::secondary("secondary", "New folder").leading(Icon::Plus))
        .child(Button::icon("icon", Icon::More, "More"))
        .child(Button::round("round", Icon::Plus, "Send"))
        .child(Button::float("float", Icon::Chevron, "Back to the end"))
        .child(Segmented::new(
            "mode",
            "Mode",
            [("ask", "Ask"), ("auto", "Auto")],
            0,
        ))
        .child(Select::new(
            "kind",
            [
                SelectOption::new("one", "One"),
                SelectOption::new("two", "Two"),
            ],
            0,
        ))
        .child(
            Select::new(
                "pill",
                [
                    SelectOption::new("one", "One"),
                    SelectOption::new("two", "Two"),
                ],
                0,
            )
            .compact(),
        )
        .child(
            Tabs::new("tabs", "Views", 0)
                .tab("all", "All")
                .tab("mine", "Mine"),
        )
        .child(FilterChips::new(
            "show",
            "Show",
            [
                FilterChip::new("all", "All", 3),
                FilterChip::new("open", "Open", 1),
            ],
            0,
        ))
        .child(Dropdown::new("picker", "Model"))
        .into_any_element()
}

fn height(id: &str, cx: &mut VisualTestContext) -> Pixels {
    bounds(id, cx).size.height
}

/// A look of hard pixels: notched corners, a 2 px border, a hard shadow, a 4 px grid, faces of
/// its own.
fn pixel() -> Look {
    Look {
        fonts: Fonts {
            ui: "Pixel Sans".into(),
            title: Some("Pixel Display".into()),
            control: Some("Pixel Mono".into()),
        },
        corners: Corners::Notch {
            step: px(2.),
            steps: 1,
        },
        border: px(2.),
        shadow: Some(Shadow {
            x: px(4.),
            y: px(4.),
            color: Role::Line2,
        }),
        grid: Some(px(4.)),
        sizes: Sizes::default(),
    }
}

// ── With no look set, every component renders exactly as before ────────────────────────────────

#[test]
fn with_no_look_set_the_look_is_the_kit_s_own_design() {
    let look = Look::default();
    assert_eq!(look.fonts.ui, SharedString::from("Geist"));
    assert_eq!(look.fonts.ui.as_ref(), gpui_design_system::typography::UI);
    assert_eq!(look.fonts.title, None, "titles in the interface face");
    assert_eq!(look.fonts.control, None, "controls in the interface face");
    assert_eq!(look.corners, Corners::Rounded);
    assert_eq!(look.border, px(1.));
    assert_eq!(look.shadow, None);
    assert_eq!(look.grid, None);
    let s = look.sizes;
    assert_eq!(
        [s.button, s.button_padding, s.icon_button, s.float_button],
        [px(30.), px(12.), px(28.), px(32.)]
    );
    assert_eq!([s.field, s.field_padding], [px(34.), px(10.)]);
    assert_eq!([s.pill, s.pill_padding], [px(24.), px(9.)]);
    assert_eq!([s.segmented, s.segment_padding], [px(26.), px(9.)]);
    assert_eq!([s.tab, s.tab_padding], [px(34.), px(10.)]);
    assert_eq!([s.chip, s.chip_padding], [px(22.), px(9.)]);
    assert_eq!(look.resolved(), look, "nothing to snap without a grid");
}

#[gpui_kit::test]
fn with_no_look_set_the_kit_draws_in_its_own_design(cx: &mut TestAppContext) {
    init(cx);
    assert_eq!(*look(), Look::default());
    // The interface face as before.
    let mut text = div().ui_text();
    assert_eq!(
        text.style().text.font_family,
        Some(SharedString::from("Geist"))
    );
    // A title and a control's words name no face of their own: they take the one around them.
    let mut title = div().title_text();
    assert_eq!(title.style().text.font_family, None);
    let mut words = div().control_text();
    assert_eq!(
        words.style().text.font_family,
        Some(SharedString::from("Geist"))
    );
}

#[gpui_kit::test]
fn with_no_look_set_every_control_keeps_its_height(cx: &mut TestAppContext) {
    init(cx);
    let (_, cx) = host(cx, controls);
    for (id, expected) in [
        ("primary", 30.),
        ("secondary", 30.),
        ("icon", 28.),
        ("round", 30.),
        ("float", 32.),
        ("mode", 26.),
        ("kind", 34.),
        ("pill", 24.),
        ("tabs", 34.),
        ("show-all", 22.),
        ("picker", 28.),
    ] {
        assert_eq!(height(id, cx), px(expected), "{id}");
    }
    assert_eq!(bounds("icon", cx).size.width, px(28.));
    assert_eq!(bounds("float", cx).size.width, px(32.));
    let (button, label) = (bounds("primary", cx), bounds("primary-label", cx));
    assert_eq!(label.origin.x - button.origin.x, px(12.), "the padding");
}

// ── An application built on the previous release compiles unchanged ────────────────────────────

/// What an application of the previous release names, used as it used it: this file compiling is
/// the check.
#[allow(dead_code)]
fn the_previous_release_s_api(cx: &mut gpui_kit::App) {
    use gpui_design_system::theme::radius;
    use gpui_design_system::typography::{BODY, MONO, UI};
    use gpui_design_system::{Identity, Palette, Theme, space};

    let identity = Identity {
        name: "Notes",
        themes: vec![Theme::new("plain", "Plain", Palette::own)],
        desktops: Vec::new(),
    };
    gpui_design_system::init(identity, cx);
    let faces: [&str; 2] = [UI, MONO];
    let _: f32 = BODY;
    let radii: [Pixels; 7] = [
        radius::ROW,
        radius::LIST_ROW,
        radius::CARD,
        radius::COMPOSER,
        radius::FIELD,
        radius::PILL,
        radius::BUTTON,
    ];
    let _ = div()
        .ui_text()
        .mono(12.)
        .medium()
        .semibold()
        .rounded(radii[0])
        .font_family(faces[0])
        .p(space::SM);
}

#[test]
fn an_application_built_on_the_previous_release_compiles_unchanged() {
    use gpui_design_system::theme::radius;
    // The radii keep their values: an application drawing its own surfaces still matches.
    assert_eq!(
        [
            radius::ROW,
            radius::LIST_ROW,
            radius::CARD,
            radius::COMPOSER,
            radius::FIELD,
            radius::PILL,
            radius::BUTTON,
        ],
        [7., 8., 12., 16., 11., 5., 7.].map(px)
    );
    assert_eq!(gpui_design_system::typography::UI, "Geist");
    assert_eq!(gpui_design_system::typography::MONO, "Geist Mono");
    let _ = the_previous_release_s_api;
}

// ── A pixel look changes every component with a corner, a border or a control height ───────────

#[gpui_kit::test]
fn a_pixel_look_gives_every_control_the_height_it_says(cx: &mut TestAppContext) {
    let sizes = Sizes {
        button: px(40.),
        icon_button: px(36.),
        float_button: px(44.),
        field: px(44.),
        pill: px(32.),
        segmented: px(36.),
        tab: px(44.),
        chip: px(32.),
        ..Sizes::default()
    };
    init_look(Look { sizes, ..pixel() }, cx);
    let (_, cx) = host(cx, controls);
    for (id, expected) in [
        ("primary", 40.),
        ("secondary", 40.),
        ("icon", 36.),
        ("round", 40.),
        ("float", 44.),
        ("mode", 36.),
        ("kind", 44.),
        ("pill", 32.),
        ("tabs", 44.),
        ("show-all", 32.),
        ("picker", 28.),
    ] {
        assert_eq!(height(id, cx), px(expected), "{id}");
    }
}

#[gpui_kit::test]
fn a_pixel_look_s_faces_are_the_ones_the_kit_s_texts_take(cx: &mut TestAppContext) {
    init_look(pixel(), cx);
    let mut text = div().ui_text();
    assert_eq!(text.style().text.font_family, Some("Pixel Sans".into()));
    let mut title = div().title_text();
    assert_eq!(title.style().text.font_family, Some("Pixel Display".into()));
    let mut words = div().control_text();
    assert_eq!(words.style().text.font_family, Some("Pixel Mono".into()));
    let mut mono = div().mono(12.);
    assert_eq!(
        mono.style().text.font_family,
        Some("Geist Mono".into()),
        "the mono face is not the look's"
    );
}

/// The calls that draw a corner, a border, a separating line, a shadow or a face: a component
/// makes them through the look (`src/look.rs`), never itself.
const SHAPE_CALLS: [&str; 14] = [
    ".rounded(",
    ".rounded_",
    // A separating line drawn as a 1 px block: the look's border width.
    ".h(px(1.))",
    ".w(px(1.))",
    ".border_1()",
    ".border_t_1()",
    ".border_b_1()",
    ".border_l_1()",
    ".border_r_1()",
    ".border_color(",
    ".border_dashed()",
    ".shadow(",
    ".shadow_sm()",
    ".font_family(",
];

/// The lines of `source` that draw a shape or a face themselves.
fn own_shapes(source: &str) -> Vec<(usize, String)> {
    source
        .lines()
        .enumerate()
        .filter(|(_, line)| {
            let code = line.trim_start();
            // The interface face by its constant: the look's is `look().fonts.ui`.
            let names_the_face = code
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .any(|word| word == "UI");
            !code.starts_with("//")
                && (names_the_face || SHAPE_CALLS.iter().any(|call| code.contains(call)))
        })
        .map(|(at, line)| (at + 1, line.trim().to_owned()))
        .collect()
}

fn sources(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
}

#[test]
fn a_shape_drawn_by_hand_is_found() {
    let found = own_shapes(
        "d.rounded(px(7.))\n// .border_1()\n.edge(Side::Top, p.line)\nlet face = font(crate::typography::UI);\nlet face = control_font();\nuse crate::typography::{MONO, UI};\n.h(px(1.)).bg(p.line)\n",
    );
    assert_eq!(
        found,
        vec![
            (1, "d.rounded(px(7.))".to_owned()),
            (4, "let face = font(crate::typography::UI);".to_owned()),
            (6, "use crate::typography::{MONO, UI};".to_owned()),
            (7, ".h(px(1.)).bg(p.line)".to_owned()),
        ]
    );
}

#[test]
fn a_pixel_look_reaches_every_component_with_a_corner_a_border_or_a_face() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    sources(&root.join("src"), &mut files);
    assert!(files.len() > 20, "the sources were found: {}", files.len());
    // The look draws the shapes; the faces and the radii are defined where they always were.
    let own = ["look.rs", "typography.rs", "theme.rs"];
    let mut offenders = Vec::new();
    for file in files {
        if own.iter().any(|name| file.ends_with(name)) {
            continue;
        }
        for (at, line) in own_shapes(&fs::read_to_string(&file).unwrap()) {
            offenders.push(format!("{}:{at}: {line}", file.display()));
        }
    }
    assert!(
        offenders.is_empty(),
        "drawn without the look:\n{}",
        offenders.join("\n")
    );
}

// ── A pressed control with a hard shadow moves onto its shadow ─────────────────────────────────

fn save(_: &mut Window, _: &mut Context<Host>) -> AnyElement {
    div()
        .p(px(20.))
        .child(Button::primary("save", "Save").on_click(|_, _, _| {}))
        .into_any_element()
}

fn press(cx: &mut VisualTestContext) {
    let at = bounds("save", cx).center();
    let none = Modifiers::default();
    cx.simulate_mouse_move(at, None, none);
    cx.simulate_mouse_down(at, MouseButton::Left, none);
}

fn release(cx: &mut VisualTestContext) {
    let at = bounds("save", cx).center();
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::default());
}

#[gpui_kit::test]
fn a_pressed_control_with_a_hard_shadow_moves_onto_its_shadow(cx: &mut TestAppContext) {
    init_look(pixel(), cx);
    let (_, cx) = host(cx, save);
    let (surface, shadow) = (bounds("save-surface", cx), bounds("save-shadow", cx));
    assert_eq!(
        surface.size, shadow.size,
        "the shadow is the surface's shape"
    );
    assert_eq!(shadow.origin.x - surface.origin.x, px(4.), "right");
    assert_eq!(shadow.origin.y - surface.origin.y, px(4.), "and down");
    let label = bounds("save-label", cx);

    press(cx);
    advance(cx, PRESS / 3);
    let midway = bounds("save-surface", cx);
    assert!(
        midway.origin.x > surface.origin.x && midway.origin.x < shadow.origin.x,
        "it moves over PRESS: {midway:?}"
    );
    advance(cx, PRESS * 2);
    assert_eq!(bounds("save-surface", cx), shadow, "it sits on its shadow");
    assert_eq!(bounds("save-shadow", cx), shadow, "the shadow stays");
    let moved = bounds("save-label", cx);
    assert_eq!(
        moved.origin.x - label.origin.x,
        px(4.),
        "its words go with it"
    );
    assert_eq!(moved.origin.y - label.origin.y, px(4.));

    release(cx);
    advance(cx, PRESS * 2);
    assert_eq!(
        bounds("save-surface", cx),
        surface,
        "released, it comes back"
    );
}

#[gpui_kit::test]
fn with_reduced_motion_a_press_jumps_onto_the_shadow(cx: &mut TestAppContext) {
    init_look(pixel(), cx);
    cx.update(|cx| cx.set_reduce_motion(true));
    let (_, cx) = host(cx, save);
    let shadow = bounds("save-shadow", cx);
    press(cx);
    cx.update(|window, cx| window.render_frame(cx));
    assert_eq!(bounds("save-surface", cx), shadow, "at once");
    assert_eq!(common::pending_frames(cx), 0, "nothing moves");
}

#[gpui_kit::test]
fn without_a_shadow_a_pressed_button_still_shrinks_a_little(cx: &mut TestAppContext) {
    init(cx);
    let (_, cx) = host(cx, save);
    let surface = bounds("save-surface", cx);
    assert!(cx.update(|window, _| window.try_find("save-shadow").is_none()));
    press(cx);
    advance(cx, PRESS * 2);
    let pressed = bounds("save-surface", cx);
    assert!(pressed.size.width < surface.size.width, "{pressed:?}");
    assert_eq!(pressed.center(), surface.center());
}

// ── With a grid set, sizes, paddings and offsets are whole multiples of its unit ───────────────

fn on_grid(what: &str, value: Pixels, unit: f32) {
    let steps = f32::from(value) / unit;
    assert!(
        (steps - steps.round()).abs() < 0.01 && steps.round() >= 1.,
        "{what}: {value:?} is not a whole multiple of {unit}"
    );
}

#[gpui_kit::test]
fn with_a_grid_set_sizes_paddings_and_offsets_are_whole_multiples_of_its_unit(
    cx: &mut TestAppContext,
) {
    let unit = 8.;
    let look = Look {
        grid: Some(px(unit)),
        shadow: Some(Shadow {
            x: px(3.),
            y: px(3.),
            color: Role::Shadow,
        }),
        ..pixel()
    };
    init_look(look, cx);
    let (_, cx) = host(cx, move |window, cx| {
        div()
            .child(controls(window, cx))
            .child(save(window, cx))
            .into_any_element()
    });
    for id in [
        "primary",
        "secondary",
        "icon",
        "round",
        "float",
        "mode",
        "kind",
        "pill",
        "tabs",
        "show-all",
        "picker",
    ] {
        on_grid(id, height(id, cx), unit);
    }
    on_grid(
        "an icon button's width",
        bounds("icon", cx).size.width,
        unit,
    );
    let (button, label) = (bounds("primary", cx), bounds("primary-label", cx));
    on_grid("a button's padding", label.origin.x - button.origin.x, unit);
    let (surface, shadow) = (bounds("save-surface", cx), bounds("save-shadow", cx));
    on_grid(
        "the shadow's offset",
        shadow.origin.x - surface.origin.x,
        unit,
    );
    on_grid(
        "the shadow's offset",
        shadow.origin.y - surface.origin.y,
        unit,
    );
    press(cx);
    advance(cx, PRESS * 2);
    let pressed = bounds("save-surface", cx);
    on_grid(
        "the press's offset",
        pressed.origin.x - surface.origin.x,
        unit,
    );
}

// ── The README documents the look, with a short example of a pixel look ────────────────────────

#[test]
fn the_readme_documents_the_look_with_a_pixel_look() {
    let readme = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"))
        .expect("the README");
    let section = readme
        .split("\n## ")
        .find(|part| part.starts_with("The look"))
        .expect("a section on the look");
    for word in [
        "init_with",
        "Look {",
        "Fonts {",
        "Corners::Notch",
        "Shadow {",
        "Role::",
        "grid: Some(",
        "add_fonts",
        "Sizes",
        "Look::default()",
    ] {
        assert!(section.contains(word), "the section says {word}");
    }
}
