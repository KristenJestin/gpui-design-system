//! `TokenField`: a multi-line text field that holds tokens beside its text, and `TokenText`, the
//! same tokens in a text that is only read (a message sent).
//!
//! A token is a chip drawn where it was put in the text (an optional glyph, a label, an optional
//! detail in the faint ink) with an opaque id that the application gives it. It is one unit:
//! the caret steps over it, Backspace or Delete removes it whole, a selection that holds part of
//! it holds all of it. In the text the field edits, a token is its label, kept as one atomic
//! range by gpui-kit's input (its inline tokens), so the caret, the selection, IME, undo and a
//! copy pasted elsewhere all work on plain text, and a token pasted outside the field is its
//! label.
//!
//! The field says, as [`TokenEvent`]s, what the person did to its tokens: one removed (by
//! Backspace, Delete, a cut or text typed over it), one that came back (a paste, an undo). What
//! the application does itself ([`TokenField::insert`], [`TokenField::remove`],
//! [`TokenField::set_value`]) is not said back to it.
//!
//! A token inserted unfolds and fades in over [`ENTER`]; one removed by Backspace, Delete or
//! [`TokenField::remove`] folds away and fades out over [`PANEL`] before its text goes (it is
//! already out of the value and said removed). One that goes with a wider selection goes with
//! that text. A token selected moves its fill over [`HOVER`]; in a `TokenText`, a token that is
//! clicked fades its hover.

use std::cell::Cell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;
use std::time::Instant;

use gpui_kit::base::input::{
    Backspace, Copy, Cut, Delete, InlineToken, InlineTokenContext, InlineTokenError, InputContent,
    InputEvent, Paste, Textarea, TextareaState,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, Bounds, ClipboardItem, Context, Div, ElementId, Entity,
    EventEmitter, Global, InteractiveElement, IntoElement, ParentElement, Pixels, Render,
    RenderOnce, SharedString, Stateful, StatefulInteractiveElement, Styled, Subscription,
    WeakEntity, Window, canvas, div, px,
};

use crate::color::mix;
use crate::field::follow_theme;
use crate::icons::{Icon, icon};
use crate::interact::pressable;
use crate::motion::{ENTER, HOVER, Motion, PANEL};
use crate::theme::palette;

/// How a token is drawn: an optional glyph, its label, an optional detail in the faint ink
/// (`Image 1` with a picture's glyph and `photo.png · 2 MB`, `/name` with a command's).
#[derive(Clone, Debug, PartialEq)]
pub struct TokenLook {
    pub glyph: Option<Icon>,
    pub label: SharedString,
    pub detail: Option<SharedString>,
}

impl TokenLook {
    /// A label without control characters, never empty.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            glyph: None,
            label: label.into(),
            detail: None,
        }
    }

    pub fn glyph(mut self, glyph: Icon) -> Self {
        self.glyph = Some(glyph);
        self
    }

    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// A token: the application's id for what it stands for, and how it is drawn. The same id may
/// be in a text more than once.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub id: SharedString,
    pub look: TokenLook,
}

impl Token {
    pub fn new(id: impl Into<SharedString>, look: TokenLook) -> Self {
        Self {
            id: id.into(),
            look,
        }
    }
}

/// A part of a text with tokens, in their order.
#[derive(Clone, Debug, PartialEq)]
pub enum Piece {
    Text(SharedString),
    Token(Token),
}

/// What the person did to the tokens of a [`TokenField`], by the token's id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenEvent {
    /// A token left the text: Backspace, Delete, a cut, text typed over it.
    Removed(SharedString),
    /// A token came into the text: a paste, an undo of its removal.
    Added(SharedString),
}

/// One token of the field, in the order of the text.
struct Shown {
    token: InlineToken,
    /// Names the token for its motion, as text moves around it.
    serial: u64,
    /// 0 to 1 as it unfolds.
    arrival: Motion,
    /// 1 to 0 while it folds away, after which its text goes.
    leaving: Option<Motion>,
    selected: Motion,
    /// Its width unfolded, measured as it is drawn.
    width: Rc<Cell<Pixels>>,
}

/// What the chip of a token draws now.
struct ChipNow {
    id: ElementId,
    look: TokenLook,
    /// How far it is shown, 0 to 1, as it unfolds or folds away.
    shown: f32,
    /// How far it is selected, 0 to 1.
    selected: f32,
    width: Rc<Cell<Pixels>>,
    running: bool,
}

/// Who changed the tokens, which says what is told and what moves.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Who {
    /// The person typed, pasted, undid: they are told, what arrives unfolds.
    Person,
    /// The application inserted or removed one: what arrives unfolds, nothing is told.
    Application,
    /// The application set the whole value: it is simply there.
    Restore,
}

/// What a copy of the field holds, for a paste into a field: the clipboard holds only the text,
/// the tokens as their labels.
#[derive(Clone)]
struct Copied {
    text: String,
    pieces: Vec<Piece>,
}

impl Global for Copied {}

pub struct TokenField {
    id: SharedString,
    input: Entity<TextareaState>,
    looks: HashMap<SharedString, TokenLook>,
    shown: Vec<Shown>,
    serial: u64,
    _changes: Subscription,
}

impl EventEmitter<TokenEvent> for TokenField {}

impl TokenField {
    /// The field `id`; its tokens are `<id>-token` numbered in the order they came. Its text is a
    /// `TextareaState` that follows the theme; [`TokenField::input`] sets the rest (a
    /// placeholder, its rows, Enter).
    pub fn new(id: impl Into<SharedString>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            let mut input = TextareaState::new(window, cx);
            follow_theme(&mut input, cx);
            input
        });
        let changes = cx.subscribe_in(&input, window, |field, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                field.sync(Who::Person, cx);
            }
        });
        Self {
            id: id.into(),
            input,
            looks: HashMap::new(),
            shown: Vec::new(),
            serial: 0,
            _changes: changes,
        }
    }

    /// The text being edited, for what the field does not say itself (focus, a placeholder, its
    /// rows). Its tokens are changed through the field, never on it.
    pub fn input(&self) -> &Entity<TextareaState> {
        &self.input
    }

    /// Puts `token` in place of the selection, at the caret when nothing is selected; the caret
    /// is after it. No space is added around it.
    pub fn insert(
        &mut self,
        token: Token,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), InlineTokenError> {
        let inline = InlineToken::new(token.id.clone(), token.look.label.clone());
        self.input
            .update(cx, |input, cx| input.replace_with_token(inline, window, cx))?;
        self.looks.insert(token.id, token.look);
        self.sync(Who::Application, cx);
        Ok(())
    }

    /// Removes every token `id` of the text: they fold away, and are out of the value at once.
    pub fn remove(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let now = cx.background_executor().now();
        let reduce = cx.reduce_motion();
        for shown in &mut self.shown {
            if shown.token.id().as_ref() == id && shown.leaving.is_none() {
                shown.leaving = Some(folding(now, reduce));
            }
        }
        self.finish_leaving(window, cx);
        cx.notify();
    }

    /// The text and its tokens, in order. A token folding away is no longer in it.
    pub fn value(&self, cx: &App) -> Vec<Piece> {
        let input = self.input.read(cx);
        let text = input.value();
        let mut pieces = Vec::new();
        let mut at = 0;
        for (span, shown) in input.tokens().iter().zip(&self.shown) {
            let range = span.range();
            push_text(&mut pieces, &text[at..range.start]);
            at = range.end;
            if shown.leaving.is_none() {
                pieces.push(Piece::Token(self.token(span.token())));
            }
        }
        push_text(&mut pieces, &text[at..]);
        pieces
    }

    /// Replaces the whole text with `pieces` (a draft restored, the field emptied once it is
    /// sent), without history and without motion.
    pub fn set_value(
        &mut self,
        pieces: impl IntoIterator<Item = Piece>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), InlineTokenError> {
        let mut text = String::new();
        let mut tokens = Vec::new();
        for piece in pieces {
            match piece {
                Piece::Text(part) => text.push_str(&part),
                Piece::Token(token) => {
                    let start = text.len();
                    text.push_str(&token.look.label);
                    tokens.push((
                        start..text.len(),
                        InlineToken::new(token.id.clone(), token.look.label.clone()),
                    ));
                    self.looks.insert(token.id, token.look);
                }
            }
        }
        let mut content = InputContent::new(text);
        for (range, token) in tokens {
            content = content.with_token(range, token)?;
        }
        self.input
            .update(cx, |input, cx| input.set_value(content, window, cx));
        self.shown.clear();
        self.sync(Who::Restore, cx);
        Ok(())
    }

    fn token(&self, token: &InlineToken) -> Token {
        let look = self
            .looks
            .get(token.id())
            .cloned()
            .unwrap_or_else(|| TokenLook::new(token.label().clone()));
        Token::new(token.id().clone(), look)
    }

    /// Follows the tokens of the text: an edit replaces one stretch of the text, so what is the
    /// same before and after it keeps its place, and what differs between left or came.
    fn sync(&mut self, change: Who, cx: &mut Context<Self>) {
        let now = cx.background_executor().now();
        let reduce = cx.reduce_motion();
        let tokens: Vec<InlineToken> = self
            .input
            .read(cx)
            .tokens()
            .iter()
            .map(|span| span.token().clone())
            .collect();
        let same = |shown: &Shown, token: &InlineToken| &shown.token == token;
        let before = self
            .shown
            .iter()
            .zip(&tokens)
            .take_while(|(shown, token)| same(shown, token))
            .count();
        let after = self
            .shown
            .iter()
            .rev()
            .zip(tokens.iter().rev())
            .take(self.shown.len().min(tokens.len()) - before)
            .take_while(|(shown, token)| same(shown, token))
            .count();
        if before + after == self.shown.len() && self.shown.len() == tokens.len() {
            return;
        }
        let came: Vec<Shown> = tokens[before..tokens.len() - after]
            .iter()
            .map(|token| {
                self.serial += 1;
                let mut arrival = Motion::settled(1.0);
                if change != Who::Restore {
                    arrival.play(0.0, 1.0, std::time::Duration::ZERO, ENTER, now, reduce);
                }
                Shown {
                    token: token.clone(),
                    serial: self.serial,
                    arrival,
                    leaving: None,
                    selected: Motion::settled(0.0),
                    width: Rc::new(Cell::new(px(0.))),
                }
            })
            .collect();
        let told: Vec<SharedString> = came.iter().map(|s| s.token.id().clone()).collect();
        let end = self.shown.len() - after;
        let gone: Vec<Shown> = self.shown.splice(before..end, came).collect();
        if change == Who::Person {
            for shown in gone.iter().filter(|shown| shown.leaving.is_none()) {
                cx.emit(TokenEvent::Removed(shown.token.id().clone()));
            }
            for id in told {
                cx.emit(TokenEvent::Added(id));
            }
        }
        cx.notify();
    }

    /// Takes out of the text the tokens that have folded away, the caret staying where it was.
    fn finish_leaving(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync(Who::Person, cx);
        let now = cx.background_executor().now();
        let spans: Vec<Range<usize>> = self
            .input
            .read(cx)
            .tokens()
            .iter()
            .map(|span| span.range())
            .collect();
        let done: Vec<Range<usize>> = self
            .shown
            .iter()
            .zip(spans)
            .filter(|(shown, _)| shown.leaving.is_some_and(|m| !m.is_running(now)))
            .map(|(_, range)| range)
            .collect();
        if done.is_empty() {
            return;
        }
        self.input.update(cx, |input, cx| {
            let mut caret = input.selected_range();
            for range in done.iter().rev() {
                input.set_selected_range(range.clone(), cx);
                input.replace("", window, cx);
                let shift = |at: usize| {
                    if at >= range.end {
                        at - range.len()
                    } else {
                        at
                    }
                };
                caret = shift(caret.start)..shift(caret.end);
            }
            input.set_selected_range(caret, cx);
        });
        self.sync(Who::Application, cx);
    }

    /// Backspace after a token, or Delete before one, folds it away instead of taking it at once.
    /// With reduced motion, the input takes it and the change says so.
    fn fold_beside(&mut self, before: bool, cx: &mut Context<Self>) {
        if cx.reduce_motion() {
            return;
        }
        self.sync(Who::Person, cx);
        let input = self.input.read(cx);
        let selected = input.selected_range();
        if !selected.is_empty() || !input.is_editable() {
            return;
        }
        let at = selected.start;
        let Some(index) = input.tokens().iter().position(|span| {
            let range = span.range();
            if before {
                range.end == at
            } else {
                range.start == at
            }
        }) else {
            return;
        };
        if self.shown[index].leaving.is_some() {
            return;
        }
        let start = input.tokens()[index].range().start;
        cx.stop_propagation();
        let now = cx.background_executor().now();
        self.shown[index].leaving = Some(folding(now, false));
        cx.emit(TokenEvent::Removed(self.shown[index].token.id().clone()));
        if before {
            self.input
                .update(cx, |input, cx| input.set_selected_range(start..start, cx));
        }
        cx.notify();
    }

    /// The selection as pieces, when it holds a token: without one, the input copies it itself.
    fn selection(&self, cx: &App) -> Option<Copied> {
        let input = self.input.read(cx);
        let selected = input.selected_range();
        let text = input.value();
        let inside: Vec<_> = input
            .tokens()
            .iter()
            .filter(|span| selected.start <= span.range().start && span.range().end <= selected.end)
            .collect();
        if selected.is_empty() || inside.is_empty() {
            return None;
        }
        let mut pieces = Vec::new();
        let mut at = selected.start;
        for span in inside {
            push_text(&mut pieces, &text[at..span.range().start]);
            pieces.push(Piece::Token(self.token(span.token())));
            at = span.range().end;
        }
        push_text(&mut pieces, &text[at..selected.end]);
        Some(Copied {
            text: text[selected].to_owned(),
            pieces,
        })
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(copied) = self.selection(cx) {
            cx.stop_propagation();
            cx.write_to_clipboard(ClipboardItem::new_string(copied.text.clone()));
            cx.set_global(copied);
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        let input = self.input.read(cx);
        if !input.is_editable() {
            return;
        }
        if let Some(copied) = self.selection(cx) {
            cx.stop_propagation();
            cx.write_to_clipboard(ClipboardItem::new_string(copied.text.clone()));
            cx.set_global(copied);
            self.input
                .update(cx, |input, cx| input.replace("", window, cx));
            self.sync(Who::Person, cx);
        }
    }

    /// A paste of what a field copied puts its tokens back; any other text is the input's.
    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        let input = self.input.read(cx);
        if !input.is_editable() {
            return;
        }
        let Some(copied) = cx.try_global::<Copied>().cloned() else {
            return;
        };
        if cx.read_from_clipboard().and_then(|item| item.text()) != Some(copied.text) {
            return;
        }
        cx.stop_propagation();
        for piece in copied.pieces {
            match piece {
                Piece::Text(text) => self
                    .input
                    .update(cx, |input, cx| input.replace(text, window, cx)),
                Piece::Token(token) => {
                    let inline = InlineToken::new(token.id.clone(), token.look.label.clone());
                    let put = self
                        .input
                        .update(cx, |input, cx| input.replace_with_token(inline, window, cx));
                    if put.is_ok() {
                        self.looks.insert(token.id, token.look);
                    }
                }
            }
        }
        self.sync(Who::Person, cx);
    }

    /// What the chip of the token at `range` draws now.
    fn chip_state(
        &mut self,
        range: Range<usize>,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> Option<ChipNow> {
        let index = self
            .input
            .read(cx)
            .tokens()
            .iter()
            .position(|span| span.range() == range)?;
        let now = cx.background_executor().now();
        let reduce = cx.reduce_motion();
        let look = self.token(&self.shown.get(index)?.token).look;
        let shown = &mut self.shown[index];
        let target = if selected { 1.0 } else { 0.0 };
        if shown.selected.target() != target {
            shown.selected.animate(target, HOVER, now, reduce);
        }
        let leaving = shown.leaving.map_or(1.0, |m| m.value(now));
        let running = shown.arrival.is_running(now)
            || shown.selected.is_running(now)
            || shown.leaving.is_some_and(|m| m.is_running(now));
        crate::seen::note(
            &ElementId::Name(format!("{}-{}", self.id, shown.token.id()).into()),
            shown.arrival.value(now) * leaving,
        );
        Some(ChipNow {
            id: ElementId::NamedInteger(format!("{}-token", self.id).into(), shown.serial),
            look,
            shown: shown.arrival.value(now) * leaving,
            selected: shown.selected.value(now),
            width: shown.width.clone(),
            running,
        })
    }

    fn moving(&self, now: Instant) -> bool {
        self.shown.iter().any(|shown| {
            shown.arrival.is_running(now) || shown.leaving.is_some_and(|m| m.is_running(now))
        })
    }
}

fn folding(now: Instant, reduce: bool) -> Motion {
    let mut motion = Motion::settled(1.0);
    motion.play(1.0, 0.0, std::time::Duration::ZERO, PANEL, now, reduce);
    motion
}

fn push_text(pieces: &mut Vec<Piece>, text: &str) {
    if text.is_empty() {
        return;
    }
    if let Some(Piece::Text(last)) = pieces.last_mut() {
        *last = format!("{last}{text}").into();
    } else {
        pieces.push(Piece::Text(text.to_owned().into()));
    }
}

impl Render for TokenField {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.finish_leaving(window, cx);
        if self.moving(cx.background_executor().now()) {
            window.request_animation_frame();
        }
        let field = cx.entity().downgrade();
        div()
            .id(ElementId::Name(self.id.clone()))
            .w_full()
            .capture_action(cx.listener(|field, _: &Backspace, _, cx| field.fold_beside(true, cx)))
            .capture_action(cx.listener(|field, _: &Delete, _, cx| field.fold_beside(false, cx)))
            .capture_action(cx.listener(Self::copy))
            .capture_action(cx.listener(Self::cut))
            .capture_action(cx.listener(Self::paste))
            .child(
                Textarea::new(&self.input)
                    .token(move |token, window, cx| field_chip(&field, token, window, cx)),
            )
    }
}

/// The chip of a token in the field, unfolding, folding away or at rest.
fn field_chip(
    field: &WeakEntity<TokenField>,
    token: &InlineTokenContext,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let Some(state) = field
        .update(cx, |field, cx| {
            field.chip_state(token.range(), token.is_selected(), cx)
        })
        .ok()
        .flatten()
    else {
        return div().into_any_element();
    };
    let ChipNow {
        id,
        look,
        shown,
        selected,
        width,
        running,
    } = state;
    if running {
        window.request_animation_frame();
    }
    let p = palette(cx);
    let measured = width.clone();
    let chip = chip(id, &look, token.line_height(), cx)
        .relative()
        .bg(mix(p.card2, p.sel, selected))
        .border_color(mix(p.line, p.acc, selected))
        .child(
            canvas(
                move |bounds: Bounds<Pixels>, _, _| measured.set(bounds.size.width),
                |_, _, _, _| {},
            )
            .absolute()
            .size_full(),
        );
    if shown >= 1.0 {
        return chip.into_any_element();
    }
    // The text around it moves as it unfolds: the field lays its tokens out at the width they
    // are drawn.
    div()
        .relative()
        .overflow_hidden()
        .h(token.line_height())
        .w(width.get() * shown)
        .opacity(shown)
        .child(chip.absolute().top_0().left_0())
        .into_any_element()
}

/// The chip both draw: its glyph, its label, its detail, `height` high.
fn chip(id: ElementId, look: &TokenLook, height: Pixels, cx: &App) -> Stateful<Div> {
    let p = palette(cx);
    div()
        .id(id)
        .aria_label(look.label.clone())
        .flex()
        .flex_none()
        .items_center()
        .gap(px(4.))
        .h(height)
        .px(px(6.))
        .rounded(px(5.))
        .border_1()
        .text_color(p.text)
        .whitespace_nowrap()
        .when_some(look.glyph, |chip, glyph| {
            chip.child(icon(glyph).size(px(12.)).text_color(p.faint))
        })
        .child(look.label.clone())
        .when_some(look.detail.clone(), |chip, detail| {
            chip.child(div().text_color(p.faint).child(detail))
        })
}

type OnToken = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// A text with tokens that is only read (a message sent): the same chips as in the field, where
/// they were put, the text wrapping around them. Its tokens are `<id>-token` numbered from 0 in
/// their order; with [`TokenText::on_token_click`] they are clicked, and fade their hover.
#[derive(IntoElement)]
pub struct TokenText {
    id: SharedString,
    pieces: Vec<Piece>,
    on_click: Option<OnToken>,
}

impl TokenText {
    pub fn new(id: impl Into<SharedString>, pieces: impl IntoIterator<Item = Piece>) -> Self {
        Self {
            id: id.into(),
            pieces: pieces.into_iter().collect(),
            on_click: None,
        }
    }

    /// Told the id of a token clicked.
    pub fn on_token_click(
        mut self,
        f: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
}

/// The height of a line of a `TokenText`, and of its chips.
const LINE: f32 = 22.;

impl RenderOnce for TokenText {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let mut lines: Vec<Vec<AnyElement>> = vec![Vec::new()];
        let mut tokens: u64 = 0;
        for piece in self.pieces {
            match piece {
                Piece::Text(text) => {
                    for (at, line) in text.split('\n').enumerate() {
                        if at > 0 {
                            lines.push(Vec::new());
                        }
                        let words = lines.last_mut().expect("a line");
                        words.extend(words_of(line).map(|word| {
                            div()
                                .h(px(LINE))
                                .flex()
                                .items_center()
                                .whitespace_nowrap()
                                .child(word.to_owned())
                                .into_any_element()
                        }));
                    }
                }
                Piece::Token(token) => {
                    let id = ElementId::NamedInteger(format!("{}-token", self.id).into(), tokens);
                    tokens += 1;
                    let chip = chip(id, &token.look, px(LINE - 2.), cx)
                        .border_color(p.line)
                        .bg(p.card2);
                    let element = match &self.on_click {
                        Some(on_click) => {
                            let on_click = on_click.clone();
                            let clicked = token.id.clone();
                            let chip = chip
                                .cursor_pointer()
                                .on_click(move |_, window, cx| on_click(&clicked, window, cx));
                            pressable(chip, move |chip, hover, _| {
                                chip.bg(mix(p.card2, p.hover, hover))
                                    .border_color(mix(p.line, p.line2, hover))
                            })
                            .into_any_element()
                        }
                        None => chip.into_any_element(),
                    };
                    lines.last_mut().expect("a line").push(
                        div()
                            .h(px(LINE))
                            .flex()
                            .items_center()
                            .child(element)
                            .into_any_element(),
                    );
                }
            }
        }
        div()
            .id(ElementId::Name(self.id))
            .flex()
            .flex_col()
            .children(lines.into_iter().map(|line| {
                div()
                    .min_h(px(LINE))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .children(line)
            }))
    }
}

/// The words of `line`, each with the spaces after it, so that the line wraps between them.
fn words_of(line: &str) -> impl Iterator<Item = &str> {
    let mut rest = line;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let word = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let end = rest[word..]
            .find(|c: char| !c.is_whitespace())
            .map_or(rest.len(), |spaces| word + spaces);
        let (first, after) = rest.split_at(end);
        rest = after;
        Some(first)
    })
}
