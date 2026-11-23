//! Every component and every state of `gpui-design-system` on one page, still: nothing needs a
//! pointer to be seen, so the page can be captured in a headless compositor.
//!
//! ```text
//! cargo run -p gpui-design-system --example gallery [-- --os-release <path>] [--drawer] [--pixel] [--menu]
//! ```
//!
//! The theme is chosen as an application's is: the desktop's when the kit has a source for this
//! desktop and its theme can be used (`--os-release` points at another file than
//! `/etc/os-release`), the kit's own look otherwise. `--drawer` shows the right drawer open.
//! `--pixel` draws the kit in a pixel look: notched corners, a 2 px border, a hard shadow, a
//! 4 px grid, the controls' words in the mono face. `--menu` puts the caret in the chip picker's
//! field, which opens its menu.

use std::path::PathBuf;

use gpui_design_system::look::Role;
use gpui_design_system::{
    Badge, Button, Card, ChipOption, ChipPicker, Corners, Drawer, Fold, Fonts, Icon, Icons,
    Identity, ListRow, Look, MachineMark, MachineState, MarkSize, Piece, PulsingLabel, QrCode,
    RemovableChip, Shadow, Side, Sizes, Token, TokenField, TokenLook, TokenText, TreeRow, Type,
    enter, follow_theme, icon, meta, palette, section_label, tree_children, typography,
};
use gpui_kit::base::input::InputState;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, Bounds, Context, Entity, Focusable as _, FontWeight, Hsla,
    IntoElement, ParentElement, Render, SharedString, Styled, Window, WindowBounds, WindowOptions,
    div, hsla, px, size,
};

fn main() {
    let mut os_release = None;
    let mut drawer = false;
    let mut menu = false;
    let mut look = Look::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--os-release" => os_release = Some(PathBuf::from(args.next().expect("a path"))),
            "--drawer" => drawer = true,
            "--menu" => menu = true,
            "--pixel" => look = pixel(),
            other => panic!("unknown argument {other}"),
        }
    }
    gpui_kit::application().with_assets(Icons).run(move |cx| {
        gpui_design_system::init_with(identity(os_release), look, cx);
        let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Maximized(bounds)),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| Gallery::new(drawer, menu, window, cx))
        })
        .expect("a window");
    });
}

/// A look of hard pixels, in the bundled faces.
fn pixel() -> Look {
    Look {
        fonts: Fonts {
            control: Some(typography::MONO.into()),
            ..Fonts::default()
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

/// The kit's own look: the kit is built without a desktop source.
#[cfg(not(feature = "omarchy"))]
fn identity(_os_release: Option<PathBuf>) -> Identity {
    Identity::new("Kit")
}

#[cfg(feature = "omarchy")]
/// The kit's own look, and Omarchy's theme when the os-release says `ID=omarchy`, read from
/// `$XDG_STATE_HOME/omarchy/current/theme/colors.toml`.
fn identity(os_release: Option<PathBuf>) -> Identity {
    use gpui_design_system::omarchy::{Environment, Omarchy};
    let mut env = Environment::from_system();
    if let Some(os_release) = os_release {
        env.os_release = os_release;
    }
    Identity::new("Kit").desktop(Omarchy::new(env))
}

struct Member {
    name: &'static str,
    model: &'static str,
    doing: &'static str,
    mark: Icon,
    state: &'static str,
}

struct Gallery {
    members: Vec<Member>,
    thinking: Entity<PulsingLabel>,
    open_fold: Entity<Fold>,
    shut_fold: Entity<Fold>,
    drawer: Entity<Drawer>,
    draft: Entity<TokenField>,
    picker_input: Entity<InputState>,
    picked: Vec<SharedString>,
}

/// What the chip picker offers: a key, a label and the line under it.
const SERVICES: [(&str, &str); 5] = [
    ("database", "postgres -D data/pg"),
    ("cache", "redis-server --port 6380"),
    ("queue", "queued --workers 4"),
    ("mailbox", "mailbox --smtp 127.0.0.1:1025"),
    ("storage", "storage serve data/files"),
];

/// A text with two pictures and a command among its words, as a composer holds one.
fn draft() -> Vec<Piece> {
    let picture = |n: usize, name: &str| {
        Piece::Token(Token::new(
            format!("picture-{n}"),
            TokenLook::new(format!("Image {n}"))
                .glyph(Icon::File)
                .detail(format!("{name} · 1.2 MB")),
        ))
    };
    vec![
        Piece::Token(Token::new(
            "command-review",
            TokenLook::new("/review").glyph(Icon::Slash),
        )),
        Piece::Text(" this one is fine, ".into()),
        picture(1, "header.png"),
        Piece::Text(" but in ".into()),
        picture(2, "footer.png"),
        Piece::Text(" the text overflows.".into()),
    ]
}

struct Steps;

impl Render for Steps {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        div()
            .px(px(14.))
            .py(px(8.))
            .flex()
            .flex_col()
            .gap(px(4.))
            .text_color(p.mute)
            .ui_text()
            .text_size(px(13.))
            .child("01  The timeout is set once; something overrides it.")
            .child("02  The value is in seconds, read as milliseconds.")
    }
}

struct DrawerContent {
    rows: Vec<(Icon, &'static str, &'static str, &'static str)>,
}

impl Render for DrawerContent {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        div()
            .p(px(12.))
            .flex()
            .flex_col()
            .child(section_label("Members", cx))
            .children(
                self.rows
                    .iter()
                    .enumerate()
                    .map(|(i, (mark, state, name, model))| {
                        ListRow::new(("drawer-member", i), *name)
                            .leading(icon(*mark).text_color(p.faint))
                            .detail(*model)
                            .trailing(meta(*state, cx))
                            .selected(i == 0)
                    }),
            )
    }
}

impl Gallery {
    fn new(drawer_open: bool, menu: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let member = |name, model, doing, mark, state| Member {
            name,
            model,
            doing,
            mark,
            state,
        };
        let members = vec![
            member(
                "api-tests",
                "Sonnet",
                "app-api · TTL units",
                Icon::Check,
                "completed",
            ),
            member(
                "web-form",
                "Sonnet",
                "asked lead · #3",
                Icon::Clock,
                "waiting",
            ),
            member(
                "docs-pass",
                "Haiku",
                "docs · timeout default",
                Icon::Running,
                "running",
            ),
        ];
        let thinking = cx.new(|cx| PulsingLabel::new("Thinking", true, cx));
        let steps = cx.new(|_| Steps);
        let open_fold = cx.new(|_| Fold::new("fold-open", steps.clone(), true));
        let shut_fold = cx.new(|_| Fold::new("fold-shut", steps, false));
        let rows = vec![
            (Icon::Bell, "needs you", "lead", "Opus"),
            (Icon::Clock, "waiting", "web-form", "Sonnet"),
            (Icon::Check, "completed", "docs-pass", "Haiku"),
        ];
        let content = cx.new(|_| DrawerContent { rows });
        let drawer = cx.new(|cx| {
            let mut drawer = Drawer::new("rail-drawer", Side::Right, px(290.), content);
            if drawer_open {
                drawer.open(cx);
            }
            drawer
        });
        let draft = cx.new(|cx| {
            let mut field = TokenField::new("draft", window, cx);
            field.set_value(draft(), window, cx).expect("a valid draft");
            field
        });
        let picker_input = cx.new(|cx| {
            let mut input = InputState::new(window, cx);
            follow_theme(&mut input, cx);
            input
        });
        if menu {
            let focus = picker_input.focus_handle(cx);
            window.focus(&focus, cx);
        }
        Self {
            members,
            picker_input,
            picked: vec!["database".into(), "cache".into()],
            thinking,
            open_fold,
            shut_fold,
            drawer,
            draft,
        }
    }
}

fn panel(title: &str, cx: &App) -> gpui_kit::Div {
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(section_label(title, cx))
}

fn swatch(name: &'static str, color: Hsla, cx: &App) -> AnyElement {
    let p = palette(cx);
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(3.))
        .child(
            div()
                .size(px(26.))
                .rounded(px(6.))
                .bg(color)
                .border_1()
                .border_color(p.line2),
        )
        .child(div().mono(9.5).text_color(p.faint).child(name))
        .into_any_element()
}

impl Render for Gallery {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let tokens = [
            ("bg", p.bg),
            ("rail", p.rail),
            ("side", p.side),
            ("card", p.card),
            ("card2", p.card2),
            ("line", p.line),
            ("line2", p.line2),
            ("hover", p.hover),
            ("sel", p.sel),
            ("text", p.text),
            ("mute", p.mute),
            ("faint", p.faint),
            ("acc", p.acc),
            ("run", p.run),
            ("wait", p.wait),
            ("ok", p.ok),
            ("fail", p.fail),
            ("add", p.add),
            ("del", p.del),
        ];

        let buttons = panel("Buttons", cx)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(8.))
                    .items_center()
                    .child(Button::primary("b-new", "New item").leading(Icon::Plus))
                    .child(Button::secondary("b-folder", "New folder").leading(Icon::Plus))
                    .child(Button::quiet("b-deny", "Deny"))
                    .child(Button::primary("b-once", "Allow once")),
            )
            .child(
                div()
                    .flex()
                    .gap(px(4.))
                    .items_center()
                    .child(Button::icon("i-search", Icon::Search, "Search"))
                    .child(
                        Button::icon("i-bell", Icon::Bell, "Waiting for you")
                            .badge(Badge::count(2)),
                    )
                    .child(Button::icon(
                        "i-edit",
                        Icon::OpenInEditor,
                        "Open the folder in your editor",
                    ))
                    .child(
                        Button::icon("i-settings", Icon::Settings, "Settings")
                            .badge(Badge::alert()),
                    )
                    .child(Button::icon("i-menu", Icon::Menu, "Libraries"))
                    .child(div().ml(px(8.)).child(gpui_design_system::Tooltip::view(
                        "Open the folder in your editor".into(),
                        cx,
                    ))),
            )
            .child(
                div().flex().gap(px(10.)).children(
                    Icon::ALL
                        .iter()
                        .map(|&glyph| icon(glyph).size(px(16.)).text_color(p.mute)),
                ),
            )
            .child(section_label("Tokens", cx))
            .child(
                div().w(px(300.)).flex().flex_wrap().gap(px(6.)).children(
                    tokens
                        .into_iter()
                        .map(|(name, color)| swatch(name, color, cx)),
                ),
            );

        let tree = panel("Tree", cx).child(
            div()
                .w(px(270.))
                .p(px(8.))
                .rounded(px(10.))
                .bg(p.side)
                .flex()
                .flex_col()
                .child(section_label("Libraries", cx))
                .child(TreeRow::new("t-atlas", "atlas").top().expanded(true))
                .child(tree_children(
                    [
                        TreeRow::new("t-main", "main")
                            .leading(icon(Icon::Folder).text_color(p.faint))
                            .into_any_element(),
                        tree_children(
                            [TreeRow::new("t-s1", "Release notes for 2.4")
                                .trailing(meta("2d", cx))
                                .into_any_element()],
                            cx,
                        )
                        .into_any_element(),
                        TreeRow::new("t-ws", "fix-timeout")
                            .leading(icon(Icon::Branch).text_color(p.faint))
                            .into_any_element(),
                        tree_children(
                            [
                                TreeRow::new("t-s2", "Tokens expire too early")
                                    .selected(true)
                                    .trailing(icon(Icon::Bell).text_color(p.wait))
                                    .into_any_element(),
                                TreeRow::new("t-s3", "CSV export by tenant")
                                    .trailing(icon(Icon::Running).text_color(p.run))
                                    .into_any_element(),
                                TreeRow::new("t-s4", "Review the refresh flow")
                                    .trailing(meta("1h", cx))
                                    .into_any_element(),
                            ],
                            cx,
                        )
                        .into_any_element(),
                    ],
                    cx,
                ))
                .child(
                    TreeRow::new("t-notes", "notes-app")
                        .top()
                        .expanded(false)
                        .trailing(icon(Icon::Bell).text_color(p.wait)),
                ),
        );

        let place = Card::new()
            .title(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(icon(Icon::Branch).text_color(p.faint))
                    .child("fix-timeout"),
            )
            .meta("from develop · +2 −1")
            .child(div().p(px(4.)).flex().flex_col().children(
                self.members.iter().enumerate().map(|(i, a)| {
                    enter(("enter-member", i), i, {
                        ListRow::new(("member", i), a.name)
                            .leading(icon(a.mark).text_color(p.faint))
                            .detail(a.model)
                            .meta(a.doing)
                            .trailing(meta(a.state, cx))
                    })
                }),
            ));

        let approval = Card::new()
            .attention()
            .child(
                div()
                    .px(px(16.))
                    .pt(px(14.))
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(icon(Icon::Bell).text_color(p.wait))
                            .child("Run npm run e2e?"),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(p.mute)
                            .child("The suite is the only check of the refresh timer."),
                    )
                    .child(meta("asked by web-form · forwarded by lead · #3", cx)),
            )
            .action(Button::quiet("a-deny", "Deny"))
            .action(Button::quiet("a-tree", "Allow for this tree"))
            .action(Button::primary("a-once", "Allow once"));

        let diff_line = |text: &'static str, bg: Option<Hsla>, ink: Hsla| {
            div()
                .px(px(14.))
                .mono(12.)
                .line_height(px(21.))
                .text_color(ink)
                .when_some(bg, |d, bg| d.bg(bg))
                .child(text)
        };
        let diff = Card::new()
            .title(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(icon(Icon::Repository).size(px(13.)).text_color(p.faint))
                    .child(
                        div()
                            .mono(12.5)
                            .font_weight(FontWeight::NORMAL)
                            .child("app-api/src/auth/token.ts"),
                    ),
            )
            .meta("+2 −1")
            .child(
                div()
                    .py(px(6.))
                    .child(diff_line(
                        "export function tokenTtlMs(env: Env) {",
                        None,
                        p.mute,
                    ))
                    .child(diff_line(
                        "-  return env.TOKEN_TTL / 6;",
                        Some(p.del),
                        p.fail,
                    ))
                    .child(diff_line(
                        "+  return env.TOKEN_TTL * 1000;",
                        Some(p.add),
                        p.ok,
                    ))
                    .child(diff_line("}", None, p.mute)),
            );

        let folds = Card::new()
            .title(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(self.thinking.clone())
                    .child(meta("2.0s", cx)),
            )
            .child(self.open_fold.clone())
            .child(
                div()
                    .px(px(14.))
                    .py(px(6.))
                    .border_t_1()
                    .border_color(p.line)
                    .text_color(p.faint)
                    .text_size(px(13.))
                    .child("Thought for 2.0s (folded)"),
            )
            .child(self.shut_fold.clone());

        let type_sample = panel("Type", cx).child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(24.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("atlas"),
                )
                .child(div().child("Geist 13.5 — Tokens expire after 5 minutes instead of 30."))
                .child(
                    div()
                        .mono(12.)
                        .text_color(p.mute)
                        .child("Geist Mono — ~/code/atlas · develop · 0123456789"),
                ),
        );

        let inline_tokens = panel("Inline tokens", cx)
            .child(
                div()
                    .px(px(10.))
                    .py(px(8.))
                    .rounded(px(8.))
                    .bg(p.field)
                    .border_1()
                    .border_color(p.field_line)
                    .child(self.draft.clone()),
            )
            .child(TokenText::new("sent", draft()).on_token_click(|_, _, _| {}));

        let this = cx.entity().downgrade();
        let chips = panel("Chips", cx)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .child(RemovableChip::new("g-chip-a", "api").at_rest())
                    .child(
                        RemovableChip::new("g-chip-b", "worker")
                            .detail("cargo run -p worker")
                            .at_rest(),
                    ),
            )
            .child(
                ChipPicker::new(
                    "g-picker",
                    "Needs",
                    &self.picker_input,
                    SERVICES.map(|(key, line)| ChipOption::new(key, key).detail(line)),
                    self.picked.clone(),
                )
                .placeholder("Add a need…")
                .on_change(move |keys, _, cx| {
                    this.update(cx, |gallery, cx| {
                        gallery.picked = keys;
                        cx.notify();
                    })
                    .ok();
                }),
            );

        let machine_marks = {
            let states = [
                (MachineState::Here, "here", "this machine"),
                (MachineState::Online, "online", "online"),
                (MachineState::Offline, "offline", "offline since 09:33"),
                (MachineState::Update, "update", "needs an update"),
                (MachineState::Unknown, "unknown", "not tried yet"),
            ];
            let sizes = [
                (MarkSize::Large, "16"),
                (MarkSize::Medium, "14"),
                (MarkSize::Small, "12"),
            ];
            let machine = hsla(0.5, 0.65, 0.55, 1.0);
            let row = |size: MarkSize, px_size: &'static str| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(14.))
                    .child(
                        div()
                            .w(px(24.))
                            .mono(9.5)
                            .text_color(p.faint)
                            .child(px_size),
                    )
                    .children(states.iter().map(|&(state, name, words)| {
                        div().w(px(52.)).child(
                            MachineMark::new(
                                format!("mark-{px_size}-{name}"),
                                Icon::Lucide("server"),
                                machine,
                                state,
                                format!("server · {words}"),
                            )
                            .size(size),
                        )
                    }))
            };
            panel("Machine marks", cx)
                .child(
                    div()
                        .flex()
                        .pl(px(24.))
                        .children(states.iter().map(|&(_, name, _)| {
                            div().w(px(52.)).mono(9.5).text_color(p.faint).child(name)
                        })),
                )
                .children(sizes.map(|(size, px_size)| row(size, px_size)))
        };
        let qr_code = panel("QR code", cx).child(
            QrCode::new(
                "g-qr",
                "https://hub.example.test/join?invite=7GQ2M9XK4TD0VBHW5ZE8RJNC3FY61PAS",
                "Example invitation code",
            )
            .size(px(192.)),
        );

        div()
            .size_full()
            .relative()
            .bg(p.bg)
            .text_color(p.text)
            .ui_text()
            .child(
                div()
                    .size_full()
                    .p(px(24.))
                    .flex()
                    .gap(px(28.))
                    .child(
                        div()
                            .w(px(230.))
                            .flex()
                            .flex_col()
                            .gap(px(18.))
                            .child(type_sample)
                            .child(inline_tokens)
                            .child(qr_code),
                    )
                    .child(
                        div()
                            .w(px(300.))
                            .flex()
                            .flex_col()
                            .gap(px(18.))
                            .child(chips)
                            .child(machine_marks),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(18.))
                            .child(tree)
                            .child(buttons),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(12.))
                            .child(section_label("Cards and rows", cx))
                            .child(place)
                            .child(approval)
                            .child(diff)
                            .child(folds),
                    ),
            )
            .child(self.drawer.clone())
    }
}
