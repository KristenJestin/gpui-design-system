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
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AppContext as _, ClipboardItem, Context, Entity, IntoElement, ParentElement, Render,
    SharedString, Styled, TestAppContext, VisualTestContext, Window, div, px, size,
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
    let part = seen("draft-img-1").expect("the chip was drawn");
    assert!(0.0 < part && part < 1.0, "it unfolds, not a jump: {part}");
    advance(cx, ENTER * 2);
    assert_eq!(seen("draft-img-1"), Some(1.0));
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
    let part = seen("draft-img-1").unwrap();
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
    let part = seen("draft-img-3").unwrap();
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
    assert_eq!(heard(&view, cx), [TokenEvent::Added("img-1".into())]);

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

    // The same words copied elsewhere are only text.
    cx.update(|_, cx| cx.write_to_clipboard(ClipboardItem::new_string("Image 1".into())));
    let end = raw(&view, cx).len();
    select(&view, end..end, cx);
    keys("ctrl-v", cx);
    assert_eq!(
        value(&view, cx),
        [text("see "), Piece::Token(picture(1)), text(" hereImage 1")]
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
