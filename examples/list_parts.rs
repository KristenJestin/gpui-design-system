//! The parts of a long list that is reviewed row by row, on one page: rows that unfold, groups
//! that fold and give a row up to each other, a bar for the rows picked, the lines of a text that
//! changed and a folded line for the rest.
//!
//! ```text
//! cargo run --example list_parts [-- --light]
//! ```
//!
//! Click a row to unfold it, a header to fold its group, "Move" to send a row to the next group,
//! "Pick" to add it to the selection.

use std::collections::HashSet;

use gpui_design_system::{
    BulkBar, Button, DiffLine, DiffLines, Group, Grouped, Icons, Identity, Item, Summary, Type,
    Unfolding, palette,
};
use gpui_kit::{
    AppContext as _, Bounds, Context, IntoElement, ParentElement, Render, SharedString, Styled,
    Window, WindowBounds, WindowOptions, div, px, size,
};

const GROUPS: [(&str, &str, &str); 3] = [
    ("asked", "Asked", "Always a card for you."),
    ("allowed", "Allowed", "Runs without asking."),
    ("judged", "Judged", "The judge decides."),
];

const ROWS: [(&str, &str, &str); 5] = [
    ("fetch_page", "judged", "Fetch a page and return its text."),
    ("save_file", "judged", "Write a file inside the folder."),
    ("list_tabs", "allowed", "List the tabs that are open."),
    (
        "run_script",
        "asked",
        "Run a script in the page and return what it returned.",
    ),
    ("clear_cache", "judged", "Empty the cache of the browser."),
];

fn main() {
    let light = std::env::args().skip(1).any(|arg| arg == "--light");
    gpui_kit::application().with_assets(Icons).run(move |cx| {
        gpui_design_system::init(Identity::new("List parts"), cx);
        if light {
            gpui_design_system::appearance::appear(
                gpui_design_system::Appearance {
                    source: gpui_design_system::Source::Theme("own".into()),
                    shade: gpui_design_system::theme::Shade::Light,
                },
                cx,
            );
        }
        let bounds = Bounds::centered(None, size(px(760.), px(900.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |_, cx| cx.new(|_| Page::new())).expect("a window");
    });
}

struct Page {
    /// Each row's name, group and sentence.
    rows: Vec<(&'static str, &'static str, &'static str)>,
    grouped: Grouped<&'static str, usize>,
    unfolded: HashSet<usize>,
    folded: HashSet<&'static str>,
    picked: HashSet<usize>,
    rest: bool,
}

impl Page {
    fn new() -> Self {
        Self {
            rows: ROWS.to_vec(),
            grouped: Grouped::default(),
            unfolded: HashSet::new(),
            folded: HashSet::new(),
            picked: HashSet::new(),
            rest: false,
        }
    }
}

impl Render for Page {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(cx);
        let now = cx.background_executor().now();
        self.grouped.sync(
            GROUPS.map(|(group, ..)| group),
            self.rows
                .iter()
                .enumerate()
                .map(|(at, (name, group, _))| (name.to_string(), *group, at)),
            now,
            cx.reduce_motion(),
        );
        let this = cx.entity().downgrade();
        let mut list = div().flex().flex_col();
        for ((group, rows), (_, title, sentence)) in self.grouped.groups().zip(GROUPS) {
            let toggled = this.clone();
            let items = rows.rows().map(|row| {
                let at = *row.value;
                let (name, _, said) = self.rows[at];
                let (opened, picked) = (self.unfolded.contains(&at), self.picked.contains(&at));
                let (on_toggle, on_move, on_pick) = (cx.entity(), cx.entity(), cx.entity());
                let next = GROUPS[(GROUPS.iter().position(|(g, ..)| g == group).unwrap() + 1) % 3].0;
                Item::new(
                    format!("{group}-{name}"),
                    row.placement,
                    Unfolding::new(format!("row-{group}-{name}"), opened)
                        .leading(Button::choice(format!("pick-{name}"), "Pick", picked).on_click(
                            move |_, _, cx| {
                                on_pick.update(cx, |page, cx| {
                                    if !page.picked.remove(&at) {
                                        page.picked.insert(at);
                                    }
                                    cx.notify();
                                })
                            },
                        ))
                        .line(
                            div()
                                .flex()
                                .gap(px(10.))
                                .child(div().mono(12.).child(SharedString::from(name)))
                                .child(div().text_color(p.mute).child(said)),
                        )
                        .trailing(Button::secondary(format!("move-{name}"), "Move").on_click(
                            move |_, _, cx| {
                                on_move.update(cx, |page, cx| {
                                    page.rows[at].1 = next;
                                    cx.notify();
                                })
                            },
                        ))
                        .body(
                            div()
                                .p(px(8.))
                                .text_color(p.mute)
                                .child("What it does, in full, over as many lines as it needs: the row measures them and the rows below make room."),
                        )
                        .on_toggle(move |open, _, cx| {
                            on_toggle.update(cx, |page, cx| {
                                if open {
                                    page.unfolded.insert(at);
                                } else {
                                    page.unfolded.remove(&at);
                                }
                                cx.notify();
                            })
                        }),
                )
                .into_any_element()
            });
            list = list.child(
                Group::new(*group, title, rows.len())
                    .sentence(sentence)
                    .open(!self.folded.contains(group))
                    .on_toggle({
                        let group = *group;
                        move |open, _, cx| {
                            toggled
                                .update(cx, |page, cx| {
                                    if open {
                                        page.folded.remove(group);
                                    } else {
                                        page.folded.insert(group);
                                    }
                                    cx.notify();
                                })
                                .ok();
                        }
                    })
                    .children(items),
            );
        }
        let cleared = this.clone();
        let rest = this;
        div()
            .size_full()
            .bg(p.body)
            .text_color(p.text)
            .ui_text()
            .p(px(24.))
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                BulkBar::new("picked", self.picked.len())
                    .action(Button::secondary("picked-judged", "Judged"))
                    .on_clear(move |_, cx| {
                        cleared
                            .update(cx, |page, cx| {
                                page.picked.clear();
                                cx.notify();
                            })
                            .ok();
                    }),
            )
            .child(list)
            .child(
                Summary::new("rest", "31 unchanged", self.rest)
                    .detail("3 allowed · 1 asked · 1 off · 26 judged")
                    .on_toggle(move |open, _, cx| {
                        rest.update(cx, |page, cx| {
                            page.rest = open;
                            cx.notify();
                        })
                        .ok();
                    })
                    .child(DiffLines::new(
                        "diff",
                        [
                            DiffLine::kept("Drives one browser window."),
                            DiffLine::removed("Call a snapshot after a page changes."),
                            DiffLine::added("Always call a snapshot after a page changes."),
                        ],
                    )),
            )
    }
}
