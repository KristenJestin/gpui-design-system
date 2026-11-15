//! Tokens inside the text field, on GPUI's test platform: a chip put at the caret, stepped over,
//! removed whole and said, listed in order in the value, copied and cut with the text, and
//! clicked in a text that is only read.

mod common;

use std::cell::RefCell;
use std::rc::Rc;

use common::{advance, init, pending_frames};
use gpui_design_system::motion::{ENTER, HOVER, PANEL};
use gpui_design_system::seen::seen;
use gpui_design_system::{Icon, Piece, Token, TokenEvent, TokenField, TokenLook, TokenText};
use gpui_kit::base::input::InputEvent;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AppContext as _, ClipboardItem, Context, Entity, EntityInputHandler as _, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, Styled, TestAppContext, TestSupportExt,
    VisualTestContext, Window, div, px, size,
};

/// A field with what it told, as a composer holds it.
struct Composer {
    field: Entity<TokenField>,
    heard: Rc<RefCell<Vec<TokenEvent>>>,
}

impl Render for Composer {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(500.)).p(px(12.)).child(self.field.clone())
    }
}

fn composer(cx: &mut TestAppContext) -> (Entity<Composer>, &mut VisualTestContext) {
    init(cx);
    let (view, cx) = cx.add_window_view(|window, cx| {
        let field = cx.new(|cx| TokenField::new("draft", window, cx));
        let heard = Rc::new(RefCell::new(Vec::new()));
        let told = heard.clone();
        cx.subscribe(&field, move |_, _, event: &TokenEvent, _| {
            told.borrow_mut().push(event.clone())
        })
        .detach();
        Composer { field, heard }
    });
    cx.simulate_resize(size(px(600.), px(400.)));
    let input = field(&view, cx).read_with(cx, |field, _| field.input().clone());
    cx.update(|window, cx| input.update(cx, |input, cx| input.focus(window, cx)));
    cx.update(|window, cx| window.render_frame(cx));
    (view, cx)
}

fn field(view: &Entity<Composer>, cx: &mut VisualTestContext) -> Entity<TokenField> {
    view.read_with(cx, |this, _| this.field.clone())
}

fn heard(view: &Entity<Composer>, cx: &mut VisualTestContext) -> Vec<TokenEvent> {
    view.read_with(cx, |this, _| this.heard.borrow().clone())
}

fn picture(n: usize) -> Token {
    Token::new(
        format!("img-{n}"),
        TokenLook::new(format!("Image {n}"))
            .glyph(Icon::File)
            .detail("photo.png · 2 MB"),
    )
}

fn command() -> Token {
    Token::new("cmd-review", TokenLook::new("/review").glyph(Icon::Slash))
}

fn text(text: &str) -> Piece {
    Piece::Text(text.into())
}

fn set_text(view: &Entity<Composer>, value: &str, cx: &mut VisualTestContext) {
    let field = field(view, cx);
    cx.update(|window, cx| {
        field.update(cx, |field, cx| {
            field.set_value([text(value)], window, cx).unwrap()
        })
    });
    cx.update(|window, cx| window.render_frame(cx));
}

fn set_value(view: &Entity<Composer>, pieces: Vec<Piece>, cx: &mut VisualTestContext) {
    let field = field(view, cx);
    cx.update(|window, cx| {
        field.update(cx, |field, cx| field.set_value(pieces, window, cx).unwrap())
    });
    cx.update(|window, cx| window.render_frame(cx));
}

fn to_end(view: &Entity<Composer>, cx: &mut VisualTestContext) {
    let end = raw(view, cx).len();
    select(view, end..end, cx);
}

fn insert(view: &Entity<Composer>, token: Token, cx: &mut VisualTestContext) {
    let field = field(view, cx);
    cx.update(|window, cx| field.update(cx, |field, cx| field.insert(token, window, cx).unwrap()));
    cx.update(|window, cx| window.render_frame(cx));
}

fn value(view: &Entity<Composer>, cx: &mut VisualTestContext) -> Vec<Piece> {
    let field = field(view, cx);
    cx.update(|_, cx| field.read(cx).value(cx))
}

/// The text the input holds, a token as its label.
fn raw(view: &Entity<Composer>, cx: &mut VisualTestContext) -> String {
    let field = field(view, cx);
    cx.update(|_, cx| field.read(cx).input().read(cx).value().to_string())
}

fn caret(view: &Entity<Composer>, cx: &mut VisualTestContext) -> usize {
    let field = field(view, cx);
    cx.update(|_, cx| field.read(cx).input().read(cx).cursor())
}

fn select(view: &Entity<Composer>, range: std::ops::Range<usize>, cx: &mut VisualTestContext) {
    let field = field(view, cx);
    cx.update(|_, cx| {
        let input = field.read(cx).input().clone();
        input.update(cx, |input, cx| input.set_selected_range(range, cx));
    });
    cx.update(|window, cx| window.render_frame(cx));
}

fn keys(keys: &str, cx: &mut VisualTestContext) {
    cx.simulate_keystrokes(keys);
    cx.update(|window, cx| window.render_frame(cx));
}

fn clipboard(cx: &mut VisualTestContext) -> Option<String> {
    cx.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
}

#[gpui_kit::test]
fn a_token_is_inserted_at_the_caret(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    set_text(&view, "see  here", cx);
    select(&view, 4..4, cx);
    insert(&view, picture(1), cx);
    assert_eq!(
        value(&view, cx),
        [text("see "), Piece::Token(picture(1)), text(" here")]
    );
    assert_eq!(
        caret(&view, cx),
        4 + "Image 1".len(),
        "the caret is after it"
    );
    assert!(
        heard(&view, cx).is_empty(),
        "what the application did is not said back"
    );

    advance(cx, ENTER / 3);
    let part = seen("draft-img-1-0").expect("the chip was drawn");
    assert!(0.0 < part && part < 1.0, "it unfolds, not a jump: {part}");
    advance(cx, ENTER * 2);
    assert_eq!(seen("draft-img-1-0"), Some(1.0));
    assert_eq!(pending_frames(cx), 0, "and it stops asking for frames");

    cx.simulate_input("!");
    assert_eq!(
        value(&view, cx),
        [text("see "), Piece::Token(picture(1)), text("! here")],
        "what is typed next goes after it"
    );
}

#[gpui_kit::test]
fn the_caret_steps_over_a_token(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    set_text(&view, "ab", cx);
    select(&view, 1..1, cx);
    insert(&view, command(), cx);
    advance(cx, ENTER * 2);
    let after = 1 + "/review".len();
    assert_eq!(caret(&view, cx), after);

    keys("left", cx);
    assert_eq!(caret(&view, cx), 1, "left steps over the whole token");
    keys("right", cx);
    assert_eq!(caret(&view, cx), after, "and right steps back over it");
    keys("right", cx);
    assert_eq!(caret(&view, cx), after + 1);

    select(&view, 3..3, cx);
    assert_eq!(caret(&view, cx), after, "a caret never stands inside it");
    select(&view, 0..3, cx);
    let field = field(&view, cx);
    let selected = cx.update(|_, cx| field.read(cx).input().read(cx).selected_range());
    assert_eq!(
        selected,
        0..after,
        "a selection that holds part of it holds all of it"
    );
}

#[gpui_kit::test]
fn backspace_removes_a_token_whole_and_says_which(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    set_text(&view, "see ", cx);
    select(&view, 4..4, cx);
    insert(&view, picture(1), cx);
    advance(cx, ENTER * 2);

    keys("backspace", cx);
    assert_eq!(heard(&view, cx), [TokenEvent::Removed("img-1".into())]);
    assert_eq!(
        value(&view, cx),
        [text("see ")],
        "it is out of the value at once"
    );
    assert_eq!(caret(&view, cx), 4);
    advance(cx, PANEL / 3);
    let part = seen("draft-img-1-0").unwrap();
    assert!(
        0.0 < part && part < 1.0,
        "it folds away, not a jump: {part}"
    );
    advance(cx, PANEL * 2);
    assert_eq!(
        raw(&view, cx),
        "see ",
        "and its text goes once it has folded"
    );
    assert_eq!(pending_frames(cx), 0);
    assert_eq!(heard(&view, cx).len(), 1, "said once");

    keys("ctrl-z", cx);
    assert_eq!(value(&view, cx), [text("see "), Piece::Token(picture(1))]);
    assert_eq!(
        heard(&view, cx).last(),
        Some(&TokenEvent::Added("img-1".into())),
        "an undo brings it back and says so"
    );

    // Delete before a token removes it too.
    set_text(&view, "see ", cx);
    select(&view, 4..4, cx);
    insert(&view, picture(2), cx);
    advance(cx, ENTER * 2);
    select(&view, 4..4, cx);
    keys("delete", cx);
    assert_eq!(
        heard(&view, cx).last(),
        Some(&TokenEvent::Removed("img-2".into()))
    );
    advance(cx, PANEL * 2);
    assert_eq!(raw(&view, cx), "see ");

    // The application removes one by its id: it folds away, and is not said back.
    insert(&view, picture(3), cx);
    advance(cx, ENTER * 2);
    let told = heard(&view, cx).len();
    let field = field(&view, cx);
    cx.update(|window, cx| field.update(cx, |field, cx| field.remove("img-3", window, cx)));
    assert_eq!(value(&view, cx), [text("see ")]);
    advance(cx, PANEL / 3);
    let part = seen("draft-img-3-0").unwrap();
    assert!(0.0 < part && part < 1.0, "it folds away: {part}");
    advance(cx, PANEL * 2);
    assert_eq!(raw(&view, cx), "see ");
    assert_eq!(heard(&view, cx).len(), told);
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn the_value_lists_text_and_tokens_in_order(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    cx.simulate_input("Look at ");
    insert(&view, picture(1), cx);
    cx.simulate_input(" and ");
    insert(&view, picture(2), cx);
    cx.simulate_input(", then ");
    insert(&view, command(), cx);
    cx.simulate_input(" them");
    let listed = vec![
        text("Look at "),
        Piece::Token(picture(1)),
        text(" and "),
        Piece::Token(picture(2)),
        text(", then "),
        Piece::Token(command()),
        text(" them"),
    ];
    assert_eq!(value(&view, cx), listed);
    assert_eq!(
        raw(&view, cx),
        "Look at Image 1 and Image 2, then /review them"
    );

    // A value set back is the same value: a draft restored.
    let field = field(&view, cx);
    let again = listed.clone();
    cx.update(|window, cx| {
        field.update(cx, |field, cx| field.set_value(again, window, cx).unwrap())
    });
    assert_eq!(value(&view, cx), listed);
    assert!(heard(&view, cx).is_empty());
}

#[gpui_kit::test]
fn copy_and_cut_keep_the_tokens(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    set_text(&view, "see ", cx);
    select(&view, 4..4, cx);
    insert(&view, picture(1), cx);
    cx.simulate_input(" here");
    advance(cx, ENTER * 2);
    let whole = vec![text("see "), Piece::Token(picture(1)), text(" here")];

    keys("ctrl-a", cx);
    keys("ctrl-c", cx);
    assert_eq!(
        clipboard(cx).as_deref(),
        Some("see Image 1 here"),
        "outside the field, a token is its label"
    );
    let end = raw(&view, cx).len();
    select(&view, end..end, cx);
    keys("ctrl-v", cx);
    assert_eq!(
        value(&view, cx),
        [
            text("see "),
            Piece::Token(picture(1)),
            text(" heresee "),
            Piece::Token(picture(1)),
            text(" here"),
        ],
        "pasted in the field, it is a token again"
    );
    assert!(
        heard(&view, cx).is_empty(),
        "img-1 was already in the text: a second copy says nothing"
    );

    set_text(&view, "", cx);
    let field = field(&view, cx);
    let again = whole.clone();
    cx.update(|window, cx| {
        field.update(cx, |field, cx| field.set_value(again, window, cx).unwrap())
    });
    keys("ctrl-a", cx);
    keys("ctrl-x", cx);
    assert_eq!(value(&view, cx), Vec::<Piece>::new());
    assert_eq!(
        heard(&view, cx).last(),
        Some(&TokenEvent::Removed("img-1".into())),
        "a cut removes it and says so"
    );
    keys("ctrl-v", cx);
    assert_eq!(value(&view, cx), whole, "and a paste puts it back");
    assert_eq!(
        heard(&view, cx).last(),
        Some(&TokenEvent::Added("img-1".into()))
    );
}

#[gpui_kit::test]
fn the_same_words_copied_elsewhere_stay_text(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    set_value(&view, vec![text("see "), Piece::Token(picture(1))], cx);
    keys("ctrl-a", cx);
    keys("ctrl-c", cx);
    cx.update(|_, cx| cx.write_to_clipboard(ClipboardItem::new_string("see Image 1".into())));
    to_end(&view, cx);
    keys("ctrl-v", cx);
    assert_eq!(
        value(&view, cx),
        [text("see "), Piece::Token(picture(1)), text("see Image 1")],
        "another application copied the same words: they are text"
    );
    assert!(heard(&view, cx).is_empty());
}

#[gpui_kit::test]
fn a_copy_without_tokens_forgets_the_last_one(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    set_value(&view, vec![text("see "), Piece::Token(picture(1))], cx);
    keys("ctrl-a", cx);
    keys("ctrl-c", cx);
    let with_tokens = cx.update(|_, cx| cx.read_from_clipboard()).unwrap();
    select(&view, 0..3, cx);
    keys("ctrl-c", cx);
    assert_eq!(clipboard(cx).as_deref(), Some("see"));

    // The first copy comes back on the clipboard (a clipboard manager's history): the field no
    // longer holds its tokens, so it is text.
    cx.update(|_, cx| cx.write_to_clipboard(with_tokens));
    to_end(&view, cx);
    keys("ctrl-v", cx);
    assert_eq!(
        value(&view, cx),
        [text("see "), Piece::Token(picture(1)), text("see Image 1")]
    );
    assert!(heard(&view, cx).is_empty());
}

#[gpui_kit::test]
fn backspace_on_one_of_two_identical_chips_removes_only_it(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    set_value(
        &view,
        vec![Piece::Token(picture(1)), Piece::Token(picture(1))],
        cx,
    );
    let first = "Image 1".len();
    select(&view, first..first, cx);
    keys("backspace", cx);
    assert_eq!(value(&view, cx), [Piece::Token(picture(1))]);
    advance(cx, PANEL * 2);
    advance(cx, PANEL * 2);
    assert_eq!(raw(&view, cx), "Image 1", "the other copy stays");
    assert_eq!(value(&view, cx), [Piece::Token(picture(1))]);
    assert_eq!(caret(&view, cx), 0);
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn inserting_over_a_selection_says_which_chips_it_replaced(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    set_value(
        &view,
        vec![
            text("see "),
            Piece::Token(picture(1)),
            text(" and "),
            Piece::Token(picture(2)),
        ],
        cx,
    );
    keys("ctrl-a", cx);
    insert(&view, command(), cx);
    assert_eq!(value(&view, cx), [Piece::Token(command())]);
    assert_eq!(
        heard(&view, cx),
        [
            TokenEvent::Removed("img-1".into()),
            TokenEvent::Removed("img-2".into())
        ],
        "the person chose to replace them"
    );
}

#[gpui_kit::test]
fn a_second_copy_of_a_chip_says_nothing(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    set_value(&view, vec![text("see "), Piece::Token(picture(1))], cx);
    keys("ctrl-a", cx);
    keys("ctrl-c", cx);
    to_end(&view, cx);
    keys("ctrl-v", cx);
    assert_eq!(
        value(&view, cx),
        [
            text("see "),
            Piece::Token(picture(1)),
            text("see "),
            Piece::Token(picture(1))
        ]
    );
    assert!(
        heard(&view, cx).is_empty(),
        "img-1 was already there: {:?}",
        heard(&view, cx)
    );

    keys("backspace", cx);
    advance(cx, PANEL * 2);
    assert_eq!(
        value(&view, cx),
        [text("see "), Piece::Token(picture(1)), text("see ")]
    );
    assert!(
        heard(&view, cx).is_empty(),
        "img-1 is still there: {:?}",
        heard(&view, cx)
    );
}

#[gpui_kit::test]
fn removing_the_last_copy_says_removed(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    set_value(
        &view,
        vec![
            Piece::Token(picture(1)),
            text(" "),
            Piece::Token(picture(1)),
        ],
        cx,
    );
    to_end(&view, cx);
    keys("backspace", cx);
    advance(cx, PANEL * 2);
    assert!(heard(&view, cx).is_empty());
    keys("backspace", cx);
    keys("backspace", cx);
    advance(cx, PANEL * 2);
    assert_eq!(value(&view, cx), Vec::<Piece>::new());
    assert_eq!(heard(&view, cx), [TokenEvent::Removed("img-1".into())]);

    keys("ctrl-z", cx);
    assert!(value(&view, cx).contains(&Piece::Token(picture(1))));
    assert_eq!(
        heard(&view, cx).last(),
        Some(&TokenEvent::Added("img-1".into()))
    );

    // A cut of every copy says it once.
    set_value(
        &view,
        vec![
            Piece::Token(picture(2)),
            text(" "),
            Piece::Token(picture(2)),
        ],
        cx,
    );
    keys("ctrl-a", cx);
    keys("ctrl-x", cx);
    let removed: Vec<_> = heard(&view, cx)
        .into_iter()
        .filter(|event| *event == TokenEvent::Removed("img-2".into()))
        .collect();
    assert_eq!(removed.len(), 1);
}

#[gpui_kit::test]
fn typing_with_an_input_method_while_a_chip_folds_keeps_the_typing(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    set_value(&view, vec![text("see "), Piece::Token(picture(1))], cx);
    to_end(&view, cx);
    keys("backspace", cx);
    let input = field(&view, cx).read_with(cx, |field, _| field.input().clone());
    cx.update(|window, cx| {
        input.update(cx, |input, cx| {
            input.replace_and_mark_text_in_range(None, "か", None, window, cx)
        })
    });
    advance(cx, PANEL * 2);
    assert_eq!(
        raw(&view, cx),
        "see かImage 1",
        "the chip waits while something is being composed"
    );

    cx.simulate_input("か");
    advance(cx, PANEL);
    assert_eq!(raw(&view, cx), "see か", "the typing stays, the chip goes");
    assert_eq!(value(&view, cx), [text("see か")]);
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn each_copy_of_a_chip_unfolds_on_its_own(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    insert(&view, picture(1), cx);
    advance(cx, ENTER * 2);
    insert(&view, picture(1), cx);
    advance(cx, ENTER / 3);
    assert_eq!(seen("draft-img-1-0"), Some(1.0), "the first is at rest");
    let part = seen("draft-img-1-1").expect("the second was drawn");
    assert!(0.0 < part && part < 1.0, "the second unfolds: {part}");
}

#[gpui_kit::test]
fn a_second_look_under_the_same_id_replaces_the_first(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    insert(&view, picture(1), cx);
    let renamed = Token::new(
        "img-1",
        TokenLook::new("Image 1")
            .glyph(Icon::File)
            .detail("renamed.png"),
    );
    insert(&view, renamed.clone(), cx);
    assert_eq!(
        value(&view, cx),
        [Piece::Token(renamed.clone()), Piece::Token(renamed)],
        "one id, one look"
    );
}

fn relabel(view: &Entity<Composer>, id: &str, look: TokenLook, cx: &mut VisualTestContext) {
    let field = field(view, cx);
    cx.update(|_, cx| field.update(cx, |field, cx| field.relabel(id, look, cx)));
    cx.update(|window, cx| window.render_frame(cx));
}

fn selected(view: &Entity<Composer>, cx: &mut VisualTestContext) -> std::ops::Range<usize> {
    let field = field(view, cx);
    cx.update(|_, cx| field.read(cx).input().read(cx).selected_range())
}

/// The look a picture has once the ones before it are gone.
fn renumbered(id: usize, n: usize) -> Token {
    Token::new(
        format!("img-{id}"),
        TokenLook::new(format!("Image {n}"))
            .glyph(Icon::File)
            .detail("photo.png · 2 MB"),
    )
}

#[gpui_kit::test]
fn relabel_changes_a_chip_s_look_without_an_undo_step(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    for n in 1..=3 {
        insert(&view, picture(n), cx);
    }
    advance(cx, ENTER * 2);
    // The person removes chip 2; the application numbers chip 3 again.
    let after_two = raw(&view, cx).len() - "Image 3".len();
    select(&view, after_two..after_two, cx);
    keys("backspace", cx);
    advance(cx, PANEL * 2);
    relabel(&view, "img-3", renumbered(3, 2).look, cx);
    assert_eq!(
        value(&view, cx),
        [Piece::Token(picture(1)), Piece::Token(renumbered(3, 2))],
        "the chip took its new look in place"
    );

    keys("ctrl-z", cx);
    assert_eq!(
        value(&view, cx),
        [
            Piece::Token(picture(1)),
            Piece::Token(picture(2)),
            Piece::Token(renumbered(3, 2))
        ],
        "the person's undo reaches their removal, and the relabel is still there"
    );
    keys("ctrl-y", cx);
    assert_eq!(
        value(&view, cx),
        [Piece::Token(picture(1)), Piece::Token(renumbered(3, 2))],
        "and the redo is not cleared"
    );
}

#[gpui_kit::test]
fn relabel_emits_no_change_event(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    insert(&view, picture(1), cx);
    advance(cx, ENTER * 2);
    let changes = Rc::new(RefCell::new(0));
    let counted = changes.clone();
    let input = field(&view, cx).read_with(cx, |field, _| field.input().clone());
    cx.update(|_, cx| {
        cx.subscribe(&input, move |_, event: &InputEvent, _| {
            if matches!(event, InputEvent::Change) {
                *counted.borrow_mut() += 1;
            }
        })
        .detach()
    });
    let told = heard(&view, cx).len();

    relabel(&view, "img-1", renumbered(1, 7).look, cx);
    relabel(&view, "img-9", TokenLook::new("Nothing"), cx);
    assert_eq!(*changes.borrow(), 0, "the input says no change");
    assert_eq!(heard(&view, cx).len(), told, "the field says nothing");
    assert_eq!(value(&view, cx), [Piece::Token(renumbered(1, 7))]);
}

#[gpui_kit::test]
fn relabel_keeps_the_caret_and_the_selection(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    set_text(&view, "see ", cx);
    to_end(&view, cx);
    insert(&view, picture(1), cx);
    insert(&view, picture(2), cx);
    advance(cx, ENTER * 2);
    let text_before = raw(&view, cx);

    // A selection that touches a chip holds all of it: what the input made of it must stay.
    select(&view, 2..6, cx);
    let chosen = selected(&view, cx);
    assert!(chosen.end > 6, "it holds the chip: {chosen:?}");
    relabel(&view, "img-1", renumbered(1, 1).look, cx);
    assert_eq!(selected(&view, cx), chosen, "the selection stays");

    select(&view, 4..4, cx);
    relabel(&view, "img-1", renumbered(1, 5).look, cx);
    assert_eq!(selected(&view, cx), 4..4);
    assert_eq!(caret(&view, cx), 4, "the caret stays");
    assert_eq!(raw(&view, cx), text_before, "the text is not touched");
}

#[gpui_kit::test]
fn relabel_moves_the_ink_over_hover(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    insert(&view, picture(1), cx);
    advance(cx, ENTER * 2);
    assert_eq!(
        seen("draft-img-1-0-ink"),
        Some(1.0),
        "at rest it is drawn whole"
    );

    assert_eq!(
        seen("draft-img-1-0-label"),
        Some(1.0),
        "at rest it is the current label"
    );

    // Ease-out: the motion is half way (the midpoint, where the look changes) a fifth in.
    relabel(&view, "img-1", renumbered(1, 2).look, cx);
    advance(cx, HOVER / 10);
    let part = seen("draft-img-1-0-ink").unwrap();
    assert!(
        0.0 < part && part < 1.0,
        "the ink moves, not a jump: {part}"
    );
    assert_eq!(
        seen("draft-img-1-0-label"),
        Some(0.0),
        "before the midpoint the label drawn is the old one"
    );
    advance(cx, HOVER / 4);
    assert_eq!(
        seen("draft-img-1-0-label"),
        Some(1.0),
        "after the midpoint it is the new one"
    );
    advance(cx, HOVER * 2);
    assert_eq!(seen("draft-img-1-0-ink"), Some(1.0));
    assert_eq!(seen("draft-img-1-0-label"), Some(1.0));
    assert_eq!(pending_frames(cx), 0, "and nothing is redrawn at rest");

    cx.update(|_, cx| cx.set_reduce_motion(true));
    relabel(&view, "img-1", renumbered(1, 3).look, cx);
    assert_eq!(
        seen("draft-img-1-0-ink"),
        Some(1.0),
        "with reduced motion it jumps"
    );
    assert_eq!(pending_frames(cx), 0);
}

#[gpui_kit::test]
fn relabel_changes_every_chip_of_the_id(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    insert(&view, picture(1), cx);
    insert(&view, picture(2), cx);
    insert(&view, picture(1), cx);
    advance(cx, ENTER * 2);

    relabel(&view, "img-1", renumbered(1, 9).look, cx);
    assert_eq!(
        value(&view, cx),
        [
            Piece::Token(renumbered(1, 9)),
            Piece::Token(picture(2)),
            Piece::Token(renumbered(1, 9)),
        ],
        "both copies took the look, the other id kept its own"
    );
    advance(cx, HOVER / 3);
    for copy in ["draft-img-1-0-ink", "draft-img-1-2-ink"] {
        let part = seen(copy).unwrap();
        assert!(0.0 < part && part < 1.0, "{copy} moves: {part}");
    }
    assert_eq!(
        seen("draft-img-2-1-ink"),
        Some(1.0),
        "the chip of another id does not move"
    );
    advance(cx, HOVER * 2);
    for copy in ["draft-img-1-0-label", "draft-img-1-2-label"] {
        assert_eq!(seen(copy), Some(1.0), "{copy} ends on the new look");
    }
}

#[gpui_kit::test]
fn a_relabel_during_a_relabel_ends_on_the_newest_look(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    insert(&view, picture(1), cx);
    advance(cx, ENTER * 2);

    // Interrupted while the old look fades out, then while the new one fades in: the ink picks
    // up from what is drawn, never jumps.
    for (first, second) in [(HOVER / 10, HOVER / 2), (HOVER * 7 / 10, HOVER / 2)] {
        relabel(&view, "img-1", renumbered(1, 2).look, cx);
        advance(cx, first);
        let before = seen("draft-img-1-0-ink").unwrap();
        assert!(before < 1.0, "it is moving: {before}");
        relabel(&view, "img-1", renumbered(1, 3).look, cx);
        let after = seen("draft-img-1-0-ink").unwrap();
        assert!(
            (after - before).abs() < 0.05,
            "the ink does not jump: {before} then {after}"
        );
        advance(cx, second);
        advance(cx, HOVER * 2);
        assert_eq!(value(&view, cx), [Piece::Token(renumbered(1, 3))]);
        assert_eq!(seen("draft-img-1-0-ink"), Some(1.0));
        assert_eq!(
            seen("draft-img-1-0-label"),
            Some(1.0),
            "it ends on the newest look"
        );
        assert_eq!(pending_frames(cx), 0);
        // Back to the first look for the next round.
        set_value(&view, vec![Piece::Token(picture(1))], cx);
        advance(cx, ENTER * 2);
    }
}

#[gpui_kit::test]
fn a_paste_keeps_a_relabelled_look(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    for n in 1..=3 {
        insert(&view, picture(n), cx);
    }
    advance(cx, ENTER * 2);
    // Copy chip 3 while it says "Image 3".
    let end = raw(&view, cx).len();
    select(&view, end - "Image 3".len()..end, cx);
    keys("ctrl-c", cx);
    // The person removes chip 2; the application numbers chip 3 again.
    let after_two = end - "Image 3".len();
    select(&view, after_two..after_two, cx);
    keys("backspace", cx);
    advance(cx, PANEL * 2);
    relabel(&view, "img-3", renumbered(3, 2).look, cx);
    advance(cx, HOVER * 2);

    to_end(&view, cx);
    keys("ctrl-v", cx);
    advance(cx, ENTER * 2);
    assert_eq!(
        value(&view, cx),
        [
            Piece::Token(picture(1)),
            Piece::Token(renumbered(3, 2)),
            Piece::Token(renumbered(3, 2)),
        ],
        "the field's own look wins: every copy of img-3 says Image 2"
    );

    keys("ctrl-z", cx);
    assert_eq!(
        value(&view, cx),
        [Piece::Token(picture(1)), Piece::Token(renumbered(3, 2))],
        "undoing the paste leaves Image 2"
    );
}

#[gpui_kit::test]
fn a_paste_brings_a_look_for_an_id_the_field_does_not_know(cx: &mut TestAppContext) {
    let (view, cx) = composer(cx);
    set_value(&view, vec![Piece::Token(picture(1))], cx);
    keys("ctrl-a", cx);
    keys("ctrl-c", cx);
    set_value(&view, vec![text("empty")], cx);
    to_end(&view, cx);
    keys("ctrl-v", cx);
    assert_eq!(
        value(&view, cx),
        [text("empty"), Piece::Token(picture(1))],
        "the copy's look is the only one there is"
    );
}

/// A message sent, its token clicked.
struct Message {
    clicked: Rc<RefCell<Vec<SharedString>>>,
}

impl Render for Message {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let clicked = self.clicked.clone();
        div().w(px(400.)).p(px(12.)).child(
            TokenText::new(
                "message",
                [
                    text("This one is fine, but in "),
                    Piece::Token(picture(2)),
                    text(" the text overflows"),
                ],
            )
            .on_token_click(move |id, _, _| clicked.borrow_mut().push(id.clone())),
        )
    }
}

#[gpui_kit::test]
fn a_token_in_a_read_only_text_is_clickable(cx: &mut TestAppContext) {
    init(cx);
    let clicked = Rc::new(RefCell::new(Vec::new()));
    let seen_clicks = clicked.clone();
    let (_, cx) = cx.add_window_view(move |_, _| Message {
        clicked: seen_clicks,
    });
    cx.simulate_resize(size(px(600.), px(400.)));
    cx.update(|window, cx| window.render_frame(cx));
    assert_eq!(
        cx.update(|window, _| window
            .find(("message-token", 0usize))
            .label()
            .map(str::to_owned)),
        Some("Image 2".to_owned()),
        "the chip is drawn with its label"
    );

    cx.update(|window, cx| window.hover(("message-token", 0usize), cx));
    advance(cx, HOVER * 2);
    assert_eq!(pending_frames(cx), 0, "its hover fades and stops");

    cx.update(|window, cx| window.click(("message-token", 0usize), cx));
    assert_eq!(*clicked.borrow(), [SharedString::from("img-2")]);
}

/// A message whose one word is longer than its line.
struct Narrow;

impl Render for Narrow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        // The window's root takes the whole window: the text sits in a column that does not
        // stretch it.
        div().flex().flex_col().items_start().child(
            div()
                .id("narrow")
                .w(px(80.))
                .test_support()
                .child(TokenText::new(
                    "message",
                    [text(
                        "Rindfleischetikettierungsüberwachungsaufgabenübertragungsgesetz",
                    )],
                )),
        )
    }
}

#[gpui_kit::test]
fn a_word_longer_than_the_line_wraps(cx: &mut TestAppContext) {
    init(cx);
    let (_, cx) = cx.add_window_view(|_, _| Narrow);
    cx.simulate_resize(size(px(600.), px(400.)));
    cx.update(|window, cx| window.render_frame(cx));
    let narrow = cx.update(|window, _| window.find("narrow").bounds());
    assert!(
        narrow.size.height > px(30.),
        "the word goes on over more lines instead of out of the text: {narrow:?}"
    );
}
