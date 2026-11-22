//! Every interactive component shows the pointing hand over what takes its click, like a link
//! (text fields show the text cursor, which `gpui-base` sets itself). GPUI has no query for the
//! cursor an element asked for, and gpui-kit's test support does not say which listeners an
//! element has, so no test can walk a view and ask each control for it. What a scan can hold is
//! the way in, as `instant_hovers` does for hovers: every place a source wires a click or a press
//! sets the cursor close by, in the same chain, unless it is one of the few listed below.

use std::fs;
use std::path::{Path, PathBuf};

/// How many lines before a click, and after it, the cursor may be: a chain of builder calls.
const BEFORE: usize = 8;
const AFTER: usize = 3;

/// A click that is no control: the source file, the whole trimmed line, and why it needs no hand.
/// A line is matched whole and each must be the only one of its file: a new clickable element
/// of the same shape is not excused by it.
const NOT_CONTROLS: &[(&str, &str, &str)] = &[
    (
        "src/button.rs",
        "button = button.on_mouse_down(MouseButton::Left, move |_, _, cx| {",
        "hides the tooltip of a `Button` on press: the button sets its own hand",
    ),
    (
        "src/button.rs",
        ".on_mouse_down(MouseButton::Left, move |_, _, cx| {",
        "hides the tooltip of `tipped`, an element that is no button, on press",
    ),
    (
        "src/dropdown.rs",
        ".on_mouse_down(MouseButton::Left, move |_, window, cx| dismiss(window, cx))",
        "the scrim behind an open list: a click outside dismisses it, it is no control",
    ),
    (
        "src/modal.rs",
        ".on_mouse_down(MouseButton::Left, move |_, window, cx| on_scrim(window, cx))",
        "the scrim behind a modal: a click outside dismisses it, it is no control",
    ),
    (
        "src/drawer.rs",
        ".on_click(cx.listener(|this, _, _, cx| this.close(cx)))",
        "the scrim behind a drawer: a click outside dismisses it, it is no control",
    ),
    (
        "src/interact.rs",
        "element.on_mouse_down(MouseButton::Navigate(direction), move |_, window, cx| {",
        "the mouse's back and forward buttons dismiss an overlay: no pointer to show",
    ),
    (
        "src/interact.rs",
        ".on_mouse_down(MouseButton::Left, move |_, _, cx| {",
        "the tooltip state's rest on press: the element it serves sets its own hand",
    ),
    (
        "src/chip_picker.rs",
        "button.on_click(move |event, window, cx| on_remove(event, window, cx))",
        "a call on a `Button`, which sets its own hand while it can act",
    ),
    (
        "src/chip_picker.rs",
        ".on_click(move |window, cx| {",
        "a call on a `MenuItem`, which sets its own hand while it can act",
    ),
    (
        "src/confirm.rs",
        ".on_click(move |_, window, cx| {",
        "a call on a `Button`, which sets its own hand while it can act",
    ),
    (
        "src/page.rs",
        ".on_click(move |_, window, cx| on_click(window, cx));",
        "a call on a `Button`, which sets its own hand while it can act",
    ),
    (
        "src/search_field.rs",
        ".on_click(move |_, window, cx| {",
        "a call on a `Button`, which sets its own hand while it can act",
    ),
    (
        "src/modal.rs",
        ".on_click(move |_, window, cx| on_close_button(window, cx)),",
        "a call on a `Button`, which sets its own hand while it can act",
    ),
    (
        "src/modal.rs",
        ".on_click(move |_, window, cx| on_cancel(window, cx)),",
        "a call on a `Button`, which sets its own hand while it can act",
    ),
    (
        "src/bulk_bar.rs",
        "clear = clear.on_click(move |_, window, cx| on_clear(window, cx));",
        "a call on a `Button`, which sets its own hand while it can act",
    ),
    (
        "src/bulk_bar.rs",
        ".on_click(move |_, window, cx| on_select_all(window, cx))",
        "a call on a `Button`, which sets its own hand while it can act",
    ),
    (
        "src/toast.rs",
        "button.on_click(move |_, window, cx| {",
        "a call on a `Button`, which sets its own hand while it can act",
    ),
    (
        "src/menu.rs",
        ".on_click(cx.listener(|this, _, window, cx| this.toggle(window, cx)));",
        "a call on a `Button`, which sets its own hand while it can act",
    ),
    (
        "src/menu.rs",
        "None => item.on_click(move |window, cx| {",
        "a call on a `MenuItem`, which sets its own hand while it can act",
    ),
    (
        "src/reorder.rs",
        ".on_mouse_down(MouseButton::Left, move |down, _, cx| {",
        "starts the drag of a row, which is the caller's own element and sets its own hand",
    ),
    (
        "src/subpage.rs",
        "row.on_click(move |_, window, cx| on_open(Some(key.clone()), window, cx))",
        "a call on a `ListRow`, which sets its own hand while it can act",
    ),
];

/// A source without its comments, as `(line number, line)`.
fn code(source: &str) -> Vec<(usize, &str)> {
    source
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
        .map(|(at, line)| (at + 1, line))
        .collect()
}

/// The lines of `source` that wire a click or a press with no cursor set around them.
fn without_a_cursor(file: &str, source: &str) -> Vec<(usize, String)> {
    let lines: Vec<&str> = source.lines().collect();
    let sets_a_cursor = |line: &str| {
        let line = line.trim_start();
        !line.starts_with("//")
            && (line.contains("cursor_pointer()") || line.contains("cursor_text()"))
    };
    code(source)
        .into_iter()
        .filter(|(_, line)| line.contains(".on_click(") || line.contains(".on_mouse_down("))
        .filter(|(_, line)| !line.contains("stop_propagation()"))
        .filter(|(_, line)| {
            !NOT_CONTROLS
                .iter()
                .any(|(path, whole, _)| file.ends_with(path) && line.trim() == *whole)
        })
        .filter(|(at, _)| {
            let from = at.saturating_sub(1 + BEFORE);
            let to = (*at + AFTER).min(lines.len());
            !lines[from..to].iter().any(|line| sets_a_cursor(line))
        })
        .map(|(at, line)| (at, line.trim().to_owned()))
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
fn a_click_without_a_cursor_is_found() {
    let source = "div()\n    .id(\"row\")\n    .on_click(f)\n    .child(x)";
    assert_eq!(
        without_a_cursor("src/row.rs", source),
        vec![(3, ".on_click(f)".to_owned())]
    );
    let press = "div().on_mouse_down(MouseButton::Left, f)";
    assert_eq!(without_a_cursor("src/row.rs", press).len(), 1);
}

#[test]
fn a_click_with_its_pointer_or_its_text_cursor_is_not() {
    let pointer = "div()\n    .cursor_pointer()\n    .on_click(f)";
    assert!(without_a_cursor("src/row.rs", pointer).is_empty());
    let after = "row.on_click(f)\n    .cursor_pointer()";
    assert!(without_a_cursor("src/row.rs", after).is_empty());
    let text = "div().cursor_text().on_mouse_down(MouseButton::Left, f)";
    assert!(without_a_cursor("src/field.rs", text).is_empty());
}

#[test]
fn a_comment_a_stopped_click_and_a_listed_scrim_are_not() {
    let comment = "// `.on_click(f)` needs the hand\nlet x = 1;";
    assert!(without_a_cursor("src/row.rs", comment).is_empty());
    let swallowed = "div().on_click(|_, _, cx| cx.stop_propagation())";
    assert!(without_a_cursor("src/row.rs", swallowed).is_empty());
    let scrim =
        "div()\n    .on_mouse_down(MouseButton::Left, move |_, window, cx| on_scrim(window, cx))";
    assert!(without_a_cursor("src/modal.rs", scrim).is_empty());
    assert_eq!(without_a_cursor("src/other.rs", scrim).len(), 1);
}

#[test]
fn an_exception_excuses_its_own_file_only() {
    let listed = "div()\n    .on_click(move |_, window, cx| {";
    assert_eq!(without_a_cursor("src/confirm.rs", listed).len(), 0);
    let other = "div()\n    .on_click(move |_, window, cx| {\n        a();\n    })\n    .on_click(move |_, window, cx| {";
    // The shape is listed, but a file that is not the listed one gets no excuse.
    assert_eq!(without_a_cursor("src/new.rs", other).len(), 2);
}

#[test]
fn a_cursor_far_from_the_click_does_not_count() {
    let far = format!(
        "div().cursor_pointer()\n{}.on_click(f)",
        "    .child(x)\n".repeat(BEFORE + 1)
    );
    assert_eq!(without_a_cursor("src/row.rs", &far).len(), 1);
}

#[test]
fn every_interactive_component_sets_the_pointer_cursor() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    sources(&root.join("src"), &mut files);
    assert!(files.len() > 20, "the sources were found: {}", files.len());
    let mut offenders = Vec::new();
    for file in files {
        let source = fs::read_to_string(&file).unwrap();
        let name = file.strip_prefix(root).unwrap().display().to_string();
        for (at, line) in without_a_cursor(&name, &source) {
            offenders.push(format!("{name}:{at}: {line}"));
        }
    }
    assert!(
        offenders.is_empty(),
        "what takes a click shows the pointing hand (`cursor_pointer()`, only while it can act); \
         a scrim or a helper that is no control goes in NOT_CONTROLS with its reason:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn every_exception_names_exactly_one_line_of_its_file() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (path, whole, reason) in NOT_CONTROLS {
        assert!(!reason.is_empty());
        let source = fs::read_to_string(root.join(path)).unwrap();
        let found = code(&source)
            .into_iter()
            .filter(|(_, line)| line.trim() == *whole)
            .count();
        assert_eq!(
            found, 1,
            "{path} has {found} lines `{whole}`: an exception names one line, drop or narrow it"
        );
    }
}
