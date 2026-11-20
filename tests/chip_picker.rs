//! A removable chip and a picker of chips, on GPUI's test platform: a chip unfolds when it is
//! added and folds away when it is removed, its remove button says what it removes, the menu
//! lists what is not chosen yet and follows what is typed, the keys add and remove, the menu
//! opens upward without room below and over a dialog, and nothing is drawn at rest.

mod common;

use common::{advance, init, pending_frames};
use gpui_design_system::motion::{ENTER, HOVER, MODAL, PANEL, TOOLTIP};
use gpui_design_system::{
    ChipOption, ChipPicker, Modal, RemovableChip, follow_theme, forget_overlays,
};
use gpui_kit::base::input::InputState;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AppContext as _, Context, Entity, Focusable as _, IntoElement, ParentElement, Pixels, Render,
    SharedString, Styled, TestAppContext, VisualTestContext, Window, div, px, size,
};

const OPTIONS: [(&str, &str); 6] = [
    ("web", "pnpm dev --port 5173"),
    ("api", "cargo run -p ledger-api"),
    ("worker", "cargo run -p ledger-worker"),
    ("postgres", "postgres -D data/pg"),
    ("redis", "redis-server --port 6380"),
    ("mailbox", "mailbox --smtp 127.0.0.1:1025"),
];

struct Host {
    chosen: Vec<SharedString>,
    changes: Vec<Vec<SharedString>>,
    input: Entity<InputState>,
    /// The picker at the bottom of the window instead of the top.
    bottom: bool,
    /// The picker inside a dialog, open or not.
    dialog: Option<bool>,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // As the window's root does: the overlays drawn open say so again in each frame.
        forget_overlays(cx);
        let this = cx.entity().downgrade();
        let picker = ChipPicker::new(
            "picker",
            "Needs",
            &self.input,
            OPTIONS.map(|(key, line)| ChipOption::new(key, key).detail(line)),
            self.chosen.clone(),
        )
        .placeholder("Add…")
        .on_change(move |keys, _, cx| {
            this.update(cx, |host, cx| {
                host.chosen = keys.clone();
                host.changes.push(keys);
                cx.notify();
            })
            .ok();
        });
        match self.dialog {
            Some(open) => div()
                .size_full()
                .child(Modal::new("dialog", "Edit").open(open).child(picker))
                .into_any_element(),
            None if self.bottom => div()
                .size_full()
                .flex()
                .flex_col()
                .justify_end()
                .pb(px(8.))
                .px(px(40.))
                .child(picker)
                .into_any_element(),
            None => div()
                .size_full()
                .pt(px(20.))
                .px(px(40.))
                .child(picker)
                .into_any_element(),
        }
    }
}

fn host<'a>(
    chosen: &[&'static str],
    bottom: bool,
    dialog: Option<bool>,
    cx: &'a mut TestAppContext,
) -> (Entity<Host>, &'a mut VisualTestContext) {
    init(cx);
    let chosen: Vec<SharedString> = chosen.iter().map(|key| (*key).into()).collect();
    let (host, cx) = cx.add_window_view(move |window, cx| Host {
        chosen,
        changes: Vec::new(),
        input: cx.new(|cx| {
            let mut input = InputState::new(window, cx);
            follow_theme(&mut input, cx);
            input
        }),
        bottom,
        dialog,
    });
    cx.simulate_resize(size(px(600.), px(500.)));
    cx.update(|window, cx| window.render_frame(cx));
    (host, cx)
}

fn set_chosen(host: &Entity<Host>, keys: &[&'static str], cx: &mut VisualTestContext) {
    let keys: Vec<SharedString> = keys.iter().map(|key| (*key).into()).collect();
    host.update(cx, |host, cx| {
        host.chosen = keys;
        cx.notify();
    });
    cx.update(|window, cx| window.render_frame(cx));
}

fn chosen(host: &Entity<Host>, cx: &mut VisualTestContext) -> Vec<String> {
    host.read_with(cx, |host, _| {
        host.chosen.iter().map(|key| key.to_string()).collect()
    })
}

/// Puts the caret in the field, as a click on it does: the menu opens.
fn focus_field(host: &Entity<Host>, cx: &mut VisualTestContext) {
    let input = host.read_with(cx, |host, _| host.input.clone());
    cx.update(|window, cx| {
        let focus = input.focus_handle(cx);
        window.focus(&focus, cx);
        window.render_frame(cx);
    });
    advance(cx, ENTER * 2);
}

fn type_text(host: &Entity<Host>, text: &str, cx: &mut VisualTestContext) {
    let input = host.read_with(cx, |host, _| host.input.clone());
    cx.update(|window, cx| input.update(cx, |input, cx| input.set_value(text, window, cx)));
    advance(cx, ENTER * 2);
}

fn text(host: &Entity<Host>, cx: &mut VisualTestContext) -> String {
    host.read_with(cx, |host, cx| host.input.read(cx).value().to_string())
}

fn press(key: &str, cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.press(key, cx));
    cx.update(|window, cx| window.render_frame(cx));
}

fn width(id: &str, cx: &mut VisualTestContext) -> Option<Pixels> {
    let id = id.to_owned();
    cx.update(|window, _| window.try_find(id).map(|found| found.bounds().size.width))
}

fn listed(id: &str, cx: &mut VisualTestContext) -> bool {
    let id = id.to_owned();
    cx.update(|window, cx| {
        window.render_frame(cx);
        window.try_find(id).is_some_and(|found| found.visible())
    })
}

/// The keys of the items the menu lists, top to bottom.
fn menu_keys(cx: &mut VisualTestContext) -> Vec<&'static str> {
    OPTIONS
        .iter()
        .map(|(key, _)| *key)
        .filter(|key| listed(&format!("picker-item-{key}"), cx))
        .collect()
}

#[gpui_kit::test]
fn a_chip_unfolds_when_added_and_folds_when_removed(cx: &mut TestAppContext) {
    let (host, cx) = host(&["web"], false, None, cx);
    let whole = width("picker-chip-web", cx).expect("a chip of the first frame is drawn");
    assert!(whole > px(40.), "{whole:?}");
    assert_eq!(
        pending_frames(cx),
        0,
        "the chips of the first frame are at rest"
    );

    set_chosen(&host, &["web", "api"], cx);
    advance(cx, PANEL / 3);
    let part = width("picker-chip-api", cx).expect("the new chip is drawn as it unfolds");
    assert!(px(0.) < part, "it has begun: {part:?}");
    assert!(pending_frames(cx) > 0, "it is still unfolding");
    advance(cx, PANEL * 2);
    let api = width("picker-chip-api", cx).unwrap();
    assert!(api > px(40.), "it has unfolded whole: {api:?}");
    assert!(part < api, "part way before it: {part:?} then {api:?}");
    assert_eq!(
        pending_frames(cx),
        0,
        "nothing is redrawn once it has arrived"
    );
    assert_eq!(
        width("picker-chip-web", cx),
        Some(whole),
        "the first chip did not move"
    );

    set_chosen(&host, &["web"], cx);
    advance(cx, PANEL / 3);
    let folding = width("picker-chip-api", cx).expect("the removed chip is drawn while it folds");
    assert!(folding < api, "it folds: {folding:?} of {api:?}");
    assert!(pending_frames(cx) > 0);
    advance(cx, PANEL * 2);
    assert_eq!(
        width("picker-chip-api", cx),
        None,
        "gone once it has folded away"
    );
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn the_remove_button_of_a_chip_says_what_it_removes(cx: &mut TestAppContext) {
    let (host, cx) = host(&["web", "api"], false, None, cx);
    cx.update(|window, _| {
        assert_eq!(
            window.find("picker-chip-web-remove").label(),
            Some("Remove web")
        );
        assert_eq!(
            window.find("picker-chip-api-remove").label(),
            Some("Remove api")
        );
    });
    // Its tooltip says it too, once the pointer has rested on it.
    cx.update(|window, cx| window.hover("picker-chip-web-remove", cx));
    advance(cx, TOOLTIP + HOVER * 2);
    cx.update(|window, cx| {
        window.render_frame(cx);
        let tip = window.find("picker-chip-web-remove-tooltip");
        assert_eq!(tip.label(), Some("Remove web"));
    });
    cx.update(|window, cx| window.click("picker-chip-web-remove", cx));
    assert_eq!(chosen(&host, cx), ["api"]);
    assert_eq!(host.read_with(cx, |host, _| host.changes.len()), 1);
}

#[gpui_kit::test]
fn typing_filters_the_menu_of_what_is_not_chosen(cx: &mut TestAppContext) {
    let (host, cx) = host(&["web"], false, None, cx);
    assert!(menu_keys(cx).is_empty(), "closed until the field is used");
    focus_field(&host, cx);
    assert_eq!(
        menu_keys(cx),
        ["api", "worker", "postgres", "redis", "mailbox"],
        "what is chosen is not offered"
    );
    cx.update(|window, _| {
        assert_eq!(window.find("picker-item-api").label(), Some("api"));
        assert_eq!(
            window.find("picker-item-api-detail").label(),
            Some("cargo run -p ledger-api"),
            "each item says its detail"
        );
    });
    type_text(&host, "post", cx);
    assert_eq!(menu_keys(cx), ["postgres"]);
    type_text(&host, "r", cx);
    assert!(
        !menu_keys(cx).contains(&"web"),
        "a chosen key never comes back"
    );
    type_text(&host, "zzzz", cx);
    assert!(menu_keys(cx).is_empty());
    assert!(
        listed("picker-empty", cx),
        "an empty line says nothing matches"
    );
    type_text(&host, "", cx);
    assert_eq!(menu_keys(cx).len(), 5);
    assert!(!listed("picker-empty", cx));
}

#[gpui_kit::test]
fn enter_adds_the_marked_item_and_the_menu_stays_open(cx: &mut TestAppContext) {
    let (host, cx) = host(&["web"], false, None, cx);
    focus_field(&host, cx);
    // The first item is marked; the arrows move the mark.
    press("down", cx);
    press("enter", cx);
    assert_eq!(
        chosen(&host, cx),
        ["web", "worker"],
        "the second item was marked"
    );
    assert!(
        listed("picker-menu", cx),
        "the menu stays open after an add"
    );
    assert_eq!(text(&host, cx), "", "the field is emptied for the next one");
    assert!(!menu_keys(cx).contains(&"worker"));

    // Several in a row: the mark is back on the first item.
    press("enter", cx);
    assert_eq!(chosen(&host, cx), ["web", "worker", "api"]);
    press("down", cx);
    press("up", cx);
    press("up", cx);
    press("enter", cx);
    assert_eq!(
        chosen(&host, cx),
        ["web", "worker", "api", "postgres"],
        "it stops at the top"
    );
    assert!(listed("picker-menu", cx));
    assert_eq!(host.read_with(cx, |host, _| host.changes.len()), 3);

    // A click on an item adds it as well, and the menu stays open too.
    cx.update(|window, cx| window.click("picker-item-mailbox", cx));
    assert_eq!(
        chosen(&host, cx).last().map(String::as_str),
        Some("mailbox")
    );
    assert!(listed("picker-menu", cx));
}

#[gpui_kit::test]
fn escape_closes_the_menu_and_goes_no_further(cx: &mut TestAppContext) {
    let (host, cx) = host(&[], false, Some(true), cx);
    advance(cx, MODAL * 2);
    focus_field(&host, cx);
    type_text(&host, "a", cx);
    assert!(listed("picker-menu", cx));
    press("escape", cx);
    advance(cx, HOVER * 3);
    assert!(!listed("picker-menu", cx), "the menu is closed");
    assert!(listed("dialog", cx), "the dialog around it is not");
    assert_eq!(text(&host, cx), "a", "the text stays");
}

#[gpui_kit::test]
fn backspace_in_an_empty_field_removes_the_last_chip(cx: &mut TestAppContext) {
    let (host, cx) = host(&["web", "api", "redis"], false, None, cx);
    focus_field(&host, cx);
    press("backspace", cx);
    assert_eq!(chosen(&host, cx), ["web", "api"]);
    // With text in the field, Backspace edits the text and removes no chip.
    type_text(&host, "ab", cx);
    press("backspace", cx);
    assert_eq!(chosen(&host, cx), ["web", "api"]);
    assert_eq!(text(&host, cx), "a");
    type_text(&host, "", cx);
    press("backspace", cx);
    press("backspace", cx);
    assert_eq!(chosen(&host, cx), Vec::<String>::new());
    press("backspace", cx);
    assert_eq!(
        host.read_with(cx, |host, _| host.changes.len()),
        3,
        "nothing left to remove"
    );
}

#[gpui_kit::test]
fn the_menu_opens_upward_without_room_below(cx: &mut TestAppContext) {
    let (host, cx) = host(&[], true, None, cx);
    focus_field(&host, cx);
    let (menu, field, viewport) = cx.update(|window, cx| {
        window.render_frame(cx);
        (
            window.find("picker-menu").bounds(),
            window.find("picker-search").bounds(),
            window.viewport_size(),
        )
    });
    assert!(menu.bottom() <= field.top(), "{menu:?} above {field:?}");
    assert!(menu.top() >= px(0.) && menu.bottom() <= viewport.height);
}

#[gpui_kit::test]
fn the_menu_opens_downward_with_room_below(cx: &mut TestAppContext) {
    let (host, cx) = host(&[], false, None, cx);
    focus_field(&host, cx);
    let (menu, field) = cx.update(|window, cx| {
        window.render_frame(cx);
        (
            window.find("picker-menu").bounds(),
            window.find("picker-search").bounds(),
        )
    });
    assert!(menu.top() >= field.bottom(), "{menu:?} under {field:?}");
    assert_eq!(menu.size.width, field.size.width, "as wide as the field");
}

#[gpui_kit::test]
fn the_menu_is_layered_above_a_dialog(cx: &mut TestAppContext) {
    let (host, cx) = host(&["web"], false, Some(true), cx);
    advance(cx, MODAL * 2);
    focus_field(&host, cx);
    let (menu, dialog) = cx.update(|window, cx| {
        window.render_frame(cx);
        (
            window.find("picker-menu").bounds(),
            window.find("dialog").bounds(),
        )
    });
    assert!(menu.intersects(&dialog), "the menu opens over the dialog");
    cx.update(|window, cx| window.click("picker-item-api", cx));
    assert_eq!(
        chosen(&host, cx),
        ["web", "api"],
        "the item over the dialog is picked"
    );
}

#[gpui_kit::test]
fn nothing_chosen_says_none(cx: &mut TestAppContext) {
    let (host, cx) = host(&[], false, None, cx);
    cx.update(|window, _| {
        assert_eq!(window.find("picker-label").label(), Some("Needs"));
        assert_eq!(window.find("picker-none").label(), Some("None"));
    });
    assert!(
        !listed("picker-count", cx),
        "no count while nothing is chosen"
    );

    set_chosen(&host, &["web", "api"], cx);
    advance(cx, PANEL * 2);
    assert!(
        !listed("picker-none", cx),
        "the line folds away once a chip comes"
    );
    cx.update(|window, _| assert_eq!(window.find("picker-count").label(), Some("2")));

    set_chosen(&host, &[], cx);
    advance(cx, PANEL * 2);
    assert_eq!(
        cx.update(|window, _| window.find("picker-none").label().map(str::to_owned)),
        Some("None".into()),
        "and comes back when the last one leaves"
    );
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn a_chip_fades_its_hover_and_goes_back(cx: &mut TestAppContext) {
    let (_, cx) = host(&["web"], false, None, cx);
    assert_eq!(pending_frames(cx), 0);
    cx.update(|window, cx| window.hover("picker-chip-web", cx));
    cx.update(|window, cx| window.render_frame(cx));
    assert!(pending_frames(cx) > 0, "the fill is moving, not switching");
    advance(cx, HOVER * 2);
    assert_eq!(pending_frames(cx), 0, "it has moved within HOVER");
}

#[gpui_kit::test]
fn a_picker_at_rest_draws_nothing_new(cx: &mut TestAppContext) {
    let (host, cx) = host(&["web", "api"], false, None, cx);
    assert_eq!(pending_frames(cx), 0, "closed, at rest");
    advance(cx, PANEL);
    assert_eq!(pending_frames(cx), 0);
    focus_field(&host, cx);
    advance(cx, ENTER * 2);
    assert!(listed("picker-menu", cx));
    assert_eq!(pending_frames(cx), 0, "open, at rest");
    advance(cx, PANEL);
    assert_eq!(pending_frames(cx), 0, "and still, as time passes");
    press("escape", cx);
    advance(cx, ENTER * 2);
    assert_eq!(pending_frames(cx), 0, "closed again, at rest");
}

#[gpui_kit::test]
fn a_standalone_chip_unfolds_where_it_is_drawn_and_says_its_detail(cx: &mut TestAppContext) {
    struct One(bool);
    impl Render for One {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().flex().child(
                RemovableChip::new("one", "web")
                    .detail("pnpm dev")
                    .present(self.0)
                    .on_remove(|_, _| {}),
            )
        }
    }
    init(cx);
    let (view, cx) = cx.add_window_view(|_, _| One(true));
    cx.simulate_resize(size(px(400.), px(100.)));
    cx.update(|window, cx| window.render_frame(cx));
    advance(cx, PANEL / 3);
    let part = width("one", cx).expect("drawn as it unfolds");
    advance(cx, PANEL * 2);
    assert!(width("one", cx).unwrap() > part);
    cx.update(|window, _| assert_eq!(window.find("one").label(), Some("web pnpm dev")));
    view.update(cx, |one, cx| {
        one.0 = false;
        cx.notify();
    });
    advance(cx, PANEL * 2);
    assert_eq!(width("one", cx), None);
}
