//! [`Group`]: a header with a chevron, a name, a count and a sentence that folds the rows under
//! it, and that comes and goes with them; [`Grouped`]: the rows of several groups, where a row
//! that changes group leaves one and arrives in the other.
//!
//! A group is drawn while it holds a row: the one that gets its first row unfolds in, the one that
//! loses its last folds away still saying what it held. A row that moves to another group folds
//! away in the one it leaves and unfolds in the one it reaches, each over [`PANEL`](crate::motion::PANEL), so that
//! nothing jumps (a slide across headers would need the real distance between the two rows, which
//! [`Listing`] only knows for rows of one height in one list).

use std::rc::Rc;

use gpui_kit::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, Pixels,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};

use crate::color::mix;
use crate::cut::tipped_when_cut;
use crate::ids::part;
use crate::interact::hoverable;
use crate::listing::{Listing, Placement};
use crate::reveal::{presence, presence_of};
use crate::seen;
use crate::space;
use crate::theme::palette;
use crate::typography::Type;
use crate::unfolding::chevron;

type OnToggle = Rc<dyn Fn(bool, &mut Window, &mut App) + 'static>;

/// A group of rows. Its parts, for a test: the header `<id>` (it says its hover, 0 to 1), its
/// chevron `<id>-chevron`, its name `<id>-title`, its count `<id>-count` and its sentence
/// `<id>-sentence`, each labelled with its text.
///
/// `count` is how many rows it holds: at 0 the group folds away, and the header says what it held
/// until it is gone. The group is a parent of its rows, which it folds when its header is clicked.
#[derive(IntoElement)]
pub struct Group {
    id: ElementId,
    title: SharedString,
    count: usize,
    sentence: Option<SharedString>,
    open: bool,
    gap: f32,
    on_toggle: Option<OnToggle>,
    rows: Vec<AnyElement>,
}

impl Group {
    /// The group `id` called `title`, holding `count` rows.
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>, count: usize) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            count,
            sentence: None,
            open: true,
            gap: 0.0,
            on_toggle: None,
            rows: Vec::new(),
        }
    }

    /// The gap the parent flex puts between its children, which comes and goes with the group.
    pub fn gap(mut self, gap: Pixels) -> Self {
        self.gap = f32::from(gap);
        self
    }

    /// One sentence after the count, what the group is for. It is cut with its whole text at hand.
    pub fn sentence(mut self, sentence: impl Into<SharedString>) -> Self {
        self.sentence = Some(sentence.into());
        self
    }

    /// Whether its rows are shown (the default) or folded away.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Called with the state a click on the header asks for.
    pub fn on_toggle(mut self, f: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(f));
        self
    }
}

impl ParentElement for Group {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.rows.extend(elements);
    }
}

impl RenderOnce for Group {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let line = (self.count > 0).then(|| SharedString::from(self.count.to_string()));
        let Group {
            id,
            title,
            sentence,
            open,
            gap,
            on_toggle,
            rows,
            ..
        } = self;
        let shown = Shown {
            id: id.clone(),
            title,
            sentence,
            open,
            on_toggle,
            rows,
            count: SharedString::default(),
        };
        presence_of(part(&id, "group"), line, move |count| {
            Shown { count, ..shown }.into_any_element()
        })
        .gap(px(gap))
    }
}

/// The group as it is drawn while it is there, with the count it says.
#[derive(IntoElement)]
struct Shown {
    id: ElementId,
    title: SharedString,
    sentence: Option<SharedString>,
    open: bool,
    on_toggle: Option<OnToggle>,
    rows: Vec<AnyElement>,
    count: SharedString,
}

impl RenderOnce for Shown {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = palette(cx);
        let open = self.open;
        let mut header = div()
            .id(self.id.clone())
            .aria_expanded(open)
            .w_full()
            .px(space::SM)
            .pt(space::SM)
            .pb(space::XS)
            .flex()
            .items_center()
            .gap(space::SM)
            .child(chevron(part(&self.id, "chevron"), open, window, cx))
            .child(
                div()
                    .id(part(&self.id, "title"))
                    .aria_label(self.title.clone())
                    .flex_none()
                    .mono(11.)
                    .font_weight(FontWeight::MEDIUM)
                    .whitespace_nowrap()
                    .child(self.title.to_uppercase())
                    .test_support(),
            )
            .child(
                div()
                    .id(part(&self.id, "count"))
                    .aria_label(self.count.clone())
                    .flex_none()
                    .mono(11.)
                    .child(self.count)
                    .test_support(),
            );
        if let Some(sentence) = self.sentence {
            let said = div()
                .id(part(&self.id, "sentence"))
                .aria_label(sentence.clone())
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_size(px(12.))
                .child(sentence.clone());
            header = header.child(tipped_when_cut(said, sentence, window, cx));
        }
        if let Some(on_toggle) = self.on_toggle {
            header = header
                .cursor_pointer()
                .on_click(move |_, window, cx| on_toggle(!open, window, cx));
        }
        let header_id = self.id.clone();
        let header = hoverable(header, move |d, t| {
            seen::note(&header_id, t);
            d.text_color(mix(p.faint, p.text, t))
        });
        div()
            .w_full()
            .flex()
            .flex_col()
            .child(header)
            .child(presence(
                part(&self.id, "body"),
                open,
                div().flex().flex_col().children(self.rows),
            ))
    }
}

/// The rows of several groups, each group a [`Listing`]: a row that changes group is a row that
/// left one list and arrived in the other, so it folds away in the first and unfolds in the
/// second. Draw each of the [`groups`](Self::groups) as a [`Group`] and each of its rows in an
/// [`Item`](crate::Item) with the row's [`id`](GroupedRow::id): a row has one place at a time,
/// but two while it moves, and each has its own id (and so its own state) in its own group.
pub struct Grouped<G, T> {
    name: SharedString,
    lists: Vec<(G, Listing<T>)>,
}

impl<G: Clone + PartialEq, T> Grouped<G, T> {
    /// The rows of the groups called `name`, which keeps the ids of two lists of a screen apart.
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            lists: Vec::new(),
        }
    }

    /// The groups in the order they are shown, each with its rows.
    pub fn groups(&self) -> impl Iterator<Item = GroupRows<'_, G, T>> {
        self.lists
            .iter()
            .enumerate()
            .map(|(index, (group, list))| GroupRows {
                name: &self.name,
                index,
                group,
                list,
            })
    }

    /// Takes the rows as they are now, each with its key and its group, as [`Listing::sync`] takes
    /// a list. `groups` is every group there can be, in the order they are shown (which stays the
    /// same from one call to the next), also the empty ones; a row of another group is not shown.
    pub fn sync(
        &mut self,
        groups: impl IntoIterator<Item = G>,
        items: impl IntoIterator<Item = (String, G, T)>,
        now: std::time::Instant,
        reduce: bool,
    ) {
        let mut old = std::mem::take(&mut self.lists);
        let mut lists: Vec<(G, Listing<T>)> = groups
            .into_iter()
            .map(|group| {
                let at = old.iter().position(|(known, _)| *known == group);
                let list = at.map(|at| old.remove(at).1).unwrap_or_default();
                (group, list)
            })
            .collect();
        let mut rows: Vec<Vec<(String, T)>> = lists.iter().map(|_| Vec::new()).collect();
        for (key, group, value) in items {
            if let Some(at) = lists.iter().position(|(known, _)| *known == group) {
                rows[at].push((key, value));
            }
        }
        for ((_, list), rows) in lists.iter_mut().zip(rows) {
            list.sync(rows, now, reduce);
        }
        self.lists = lists;
    }
}

/// One group of a [`Grouped`]: the group and its rows.
pub struct GroupRows<'a, G, T> {
    name: &'a SharedString,
    index: usize,
    pub group: &'a G,
    list: &'a Listing<T>,
}

impl<'a, G, T> GroupRows<'a, G, T> {
    /// How many rows it holds, the ones that are folding away not counted: the count of its
    /// [`Group`].
    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Its rows, in order, those that left included while they fold away.
    pub fn rows(&self) -> impl Iterator<Item = GroupedRow<'a, T>> {
        let (name, index) = (self.name, self.index);
        self.list.rows().map(move |row| GroupedRow {
            key: row.key,
            value: row.value,
            placement: row.placement,
            id: ElementId::Name(format!("{name}-{index}-{}", row.key).into()),
        })
    }
}

/// A row of a [`Grouped`], as [`Listing`] hands it out, with the id it has in its group.
pub struct GroupedRow<'a, T> {
    pub key: &'a str,
    pub value: &'a T,
    pub placement: Placement,
    id: ElementId,
}

impl<T> GroupedRow<'_, T> {
    /// The id of the row in its group: for its [`Item`](crate::Item).
    pub fn id(&self) -> ElementId {
        self.id.clone()
    }

    /// An id for something the row holds (its own unfolding, its controls): it is in the row's
    /// group too, so that the two copies of a row that moves never share one.
    pub fn part(&self, name: &str) -> ElementId {
        part(&self.id, name)
    }
}
