//! The kit is shared by more than one application: its sources, examples and tests, comments
//! included, name nothing of what one of them is about. A scan holds it, shown a sample of the
//! wrong text first.

use std::fs;
use std::path::{Path, PathBuf};

/// What belongs to an application and not to the kit: the app's name, and what it is made of.
const APP_WORDS: [&str; 5] = ["flottille", "agent", "session", "project", "workspace"];

/// What the kit names only when it is built with the `omarchy` feature: built without it, the kit
/// names no desktop.
const OMARCHY: &str = "omarchy";

/// The attribute of what only the `omarchy` feature builds: a whole file when it is the file's
/// own (`#![..]`), the item it is put on otherwise.
const GATE: &str = "[cfg(feature = \"omarchy\")]";

/// The attribute of what is built only without the feature: the item stays, the attribute goes.
const UNGATED: &str = "#[cfg(not(feature = \"omarchy\"))]";

/// The kit's own crate name, which a `use` in an example or a test cannot avoid: it is the kit's
/// and is not counted.
const OWN_NAME: [&str; 2] = ["gpui_design_system", "gpui-design-system"];

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

/// Whether `line` has `word` at the start of a word of its own (`sub-agents` has `agent`, a
/// colour called `magenta` has not).
fn names(line: &str, word: &str) -> bool {
    line.match_indices(word).any(|(at, _)| {
        !line[..at]
            .chars()
            .next_back()
            .is_some_and(|before| before.is_alphabetic())
    })
}

/// The lines of `source` that name an application word, numbered from 1, with the word.
fn app_words(source: &str) -> Vec<(usize, &'static str)> {
    source
        .lines()
        .enumerate()
        .flat_map(|(at, line)| {
            let line = OWN_NAME
                .into_iter()
                .fold(line.to_lowercase(), |line, own| line.replace(own, ""));
            APP_WORDS
                .into_iter()
                .chain(cfg!(not(feature = "omarchy")).then_some(OMARCHY))
                .filter(move |word| names(&line, word))
                .map(move |word| (at + 1, word))
        })
        .collect()
}

/// The lines of `source` built with the `omarchy` feature on or off: off, a file gated as a whole
/// is empty, an item gated is left out, from its attribute to its last line (a line at the
/// attribute's indentation, not a comment, that closes it), and an item built only without the
/// feature loses its attribute.
fn built(source: &str, omarchy: bool) -> String {
    if omarchy {
        return source.to_string();
    }
    if source
        .lines()
        .any(|line| line.trim() == format!("#!{GATE}"))
    {
        return String::new();
    }
    let mut kept = String::new();
    let mut gated: Option<usize> = None;
    for line in source.lines() {
        let indent = line.len() - line.trim_start().len();
        let text = line.trim();
        match gated {
            Some(at) => {
                let ends = indent == at
                    && !text.starts_with("//")
                    && !text.starts_with("#[")
                    && (text.ends_with(';') || text.ends_with('}'));
                if ends {
                    gated = None;
                }
            }
            None if text == format!("#{GATE}") => gated = Some(indent),
            None if text == UNGATED => {}
            None => {
                kept.push_str(line);
                kept.push('\n');
            }
        }
    }
    kept
}

#[test]
fn an_application_word_is_found() {
    let sample = "/// A row of a session.\nlet x = 1;\nfn flottille() {}\n// The Agents tab";
    assert_eq!(
        app_words(sample),
        [(1, "session"), (3, "flottille"), (4, "agent")]
    );
    assert!(app_words("/// A row of a list.\nlet x = 1;").is_empty());
    assert!(app_words("\"magenta\", \"cyan\"").is_empty());
}

#[test]
fn the_name_of_the_kit_itself_is_not_an_application_word() {
    assert!(
        app_words("use gpui_design_system::Button;\n//! cargo run -p gpui-design-system")
            .is_empty()
    );
    assert_eq!(
        app_words("gpui_design_system::Button::new(\"New session\")"),
        [(1, "session")]
    );
}

#[test]
fn what_only_the_omarchy_feature_builds_is_left_out_of_the_scan() {
    let gated = "use x;\n#[cfg(feature = \"omarchy\")]\npub mod omarchy;\n\
                 #[cfg(feature = \"omarchy\")]\n/// Omarchy's.\nfn f() {\n    omarchy();\n}\n\
                 #[cfg(not(feature = \"omarchy\"))]\nfn g() {}\n";
    assert_eq!(built(gated, false), "use x;\nfn g() {}\n");
    assert_eq!(built(gated, true), gated);
    let file = "#![cfg(feature = \"omarchy\")]\n//! Omarchy.\nfn f() {}\n";
    assert_eq!(built(file, false), "");
}

#[cfg(not(feature = "omarchy"))]
#[test]
fn without_the_omarchy_feature_omarchy_is_an_application_word() {
    assert_eq!(app_words("/// Omarchy's theme."), [(1, "omarchy")]);
}

#[test]
fn the_kit_names_no_app_word() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for folder in ["src", "examples", "tests"] {
        sources(&root.join(folder), &mut files);
    }
    // This file holds the words, as the samples of its own scan.
    files.retain(|file| !file.ends_with("tests/no_app_word.rs"));
    assert!(files.len() > 20, "the sources were found: {}", files.len());
    let mut found = Vec::new();
    for file in files {
        let text = built(
            &fs::read_to_string(&file).unwrap(),
            cfg!(feature = "omarchy"),
        );
        for (at, word) in app_words(&text) {
            let line = text.lines().nth(at - 1).unwrap().trim();
            found.push(format!("{}:{at}: `{word}`: {line}", file.display()));
        }
    }
    assert!(
        found.is_empty(),
        "the kit names what an application is about:\n{}",
        found.join("\n")
    );
}
