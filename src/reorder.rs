//! [`Reorderable`]: a list whose rows are dragged to their place and gathered in foldable groups.
//!
//! A row is pressed and moved past a small threshold: it lifts and follows the pointer, the rows
//! around slide aside to make room, and a dashed slot shows where it lands. Let go, and it settles
//! into the slot while the list says what moved ([`Reorderable::on_move_row`]); Escape, or a drop
//! to the side of the list, slides everything back and says nothing. A group (a header with a
//! chevron, a name and a count, which folds its rows) is dragged by its header among the groups.
//!
//! The list holds no data of its own beyond the drag in progress: the caller says the groups and
//! the rows, each by a key, and is told what moved. It keeps the order, and changes it in the
//! callback, so that the next frame draws the new one; a move it does not apply slides back.
//! The groups come first, then the rows outside any group, and a group is not dragged below them.
//!
//! A group that the caller adds unfolds and fades in, one it removes folds away and fades out
//! (its heading is kept for that, with its name, count, mark and trailing element, which the
//! group gave as functions for that reason; it takes no press), and when the caller changes the
//! order of the groups or the group of a row from outside, what moved slides to its new place,
//! while a row is held too (the row held stays under the pointer). A row that is new to the list
//! is the caller's to bring in (a row that unfolds), in a group or not.
//!
//! The drag is the list's own: the pointer is followed at the window's level (not only over the
//! list), and while a row is held a transparent sheet lies over the window so that nothing under
//! the pointer hovers, nor takes the click that ends the drag.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use gpui_kit::{
    AnyElement, App, Bounds, BoxShadow, Context, CursorStyle, DispatchPhase, ElementId, Entity,
    FocusHandle, FontWeight, InteractiveElement, IntoElement, MouseButton, MouseMoveEvent,
    MouseUpEvent, ParentElement, Pixels, Point, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, canvas, deferred, div, point, px,
};

use crate::color::mix;
use crate::ids::part;
use crate::interact::{fade, hoverable};
use crate::look::{Shaped as _, Surface};
use crate::motion::{HOVER, Motion, PANEL};
use crate::reveal::{Shown, drive, presence};
use crate::seen;
use crate::space;
use crate::theme::palette;
use crate::typography::Type;
use crate::unfolding::chevron;

/// How far the pointer goes, in pixels, before a press is a drag and no longer a click.
const THRESHOLD: f64 = 4.;

type OnMoveRow = Rc<dyn Fn(&RowMove, &mut Window, &mut App)>;
type OnMoveGroup = Rc<dyn Fn(&str, usize, &mut Window, &mut App)>;
type OnToggle = Rc<dyn Fn(&str, bool, &mut Window, &mut App)>;
/// What a heading draws, built again each time it is drawn: a heading that leaves is drawn once
/// more after its group is gone, so it cannot be given as an element, which is drawn once.
type Build = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// A row that was dragged to a place: `row` goes to `group` (`None`: outside any group) and is
/// there at `index`, counted among the rows of that place once it has left its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowMove {
    pub row: SharedString,
    pub group: Option<SharedString>,
    pub index: usize,
}

/// A group of the list: a header that folds its rows, and the rows.
pub struct ReorderGroup {
    key: SharedString,
    title: SharedString,
    open: bool,
    mark: Option<Build>,
    trailing: Option<Build>,
    rows: Vec<(SharedString, AnyElement)>,
}

impl ReorderGroup {
    /// The group `key`, called `title`.
    pub fn new(key: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            open: true,
            mark: None,
            trailing: None,
            rows: Vec::new(),
        }
    }

    /// Whether its rows are shown (the default) or folded away.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// What the header says of the rows it hides, before its name, while the group is folded: it
    /// unfolds and fades in as the group folds, and folds away as it opens.
    ///
    /// It is given as a function, called each time the heading is drawn.
    pub fn mark<E: IntoElement>(
        mut self,
        build: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.mark = Some(Rc::new(move |window, cx| {
            build(window, cx).into_any_element()
        }));
        self
    }

    /// A button of its own at the right of the heading (a menu): a press on it neither starts a
    /// drag nor folds the group. Given as a function, like [`mark`](Self::mark).
    pub fn trailing<E: IntoElement>(
        mut self,
        build: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.trailing = Some(Rc::new(move |window, cx| {
            build(window, cx).into_any_element()
        }));
        self
    }

    /// A row of the group, by its key (unique in the whole list).
    pub fn row(mut self, key: impl Into<SharedString>, content: impl IntoElement) -> Self {
        self.rows.push((key.into(), content.into_any_element()));
        self
    }
}

/// The list. Its parts, for a test: a row `<id>-row-<key>`, a group `<id>-group-<key>` with its
/// header `-header`, its chevron `-chevron`, its name `-title`, its count `-count`, its mark
/// `-mark` and its rows `-body`, the slot a held row lands in `<id>-slot` and the sheet that lies
/// over the window while a row is held `<id>-sheet`.
///
/// A row is a block of its own height: the `gap` is its own space below it, which counts in the
/// place the others make for it.
#[derive(IntoElement)]
pub struct Reorderable {
    id: ElementId,
    gap: f32,
    groups: Vec<ReorderGroup>,
    rows: Vec<(SharedString, AnyElement)>,
    on_move_row: Option<OnMoveRow>,
    on_move_group: Option<OnMoveGroup>,
    on_toggle: Option<OnToggle>,
}

impl Reorderable {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            gap: 0.0,
            groups: Vec::new(),
            rows: Vec::new(),
            on_move_row: None,
            on_move_group: None,
            on_toggle: None,
        }
    }

    /// The space below each row.
    pub fn gap(mut self, gap: Pixels) -> Self {
        self.gap = f32::from(gap);
        self
    }

    /// A group, after those already added.
    pub fn group(mut self, group: ReorderGroup) -> Self {
        self.groups.push(group);
        self
    }

    /// A row outside any group, by its key, after those already added.
    pub fn row(mut self, key: impl Into<SharedString>, content: impl IntoElement) -> Self {
        self.rows.push((key.into(), content.into_any_element()));
        self
    }

    /// Called when a row was dropped in another place than its own.
    pub fn on_move_row(mut self, f: impl Fn(&RowMove, &mut Window, &mut App) + 'static) -> Self {
        self.on_move_row = Some(Rc::new(f));
        self
    }

    /// Called with the key of a group and the index it was dropped at among the groups (counted
    /// once it has left its own).
    pub fn on_move_group(
        mut self,
        f: impl Fn(&str, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_move_group = Some(Rc::new(f));
        self
    }

    /// Called with the key of a group and the state a click on its header asks for.
    pub fn on_toggle(mut self, f: impl Fn(&str, bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(f));
        self
    }
}

// ── The shape of the list and where a held row lands: plain data, no window ────────────────────

/// What moves on its own: a group's header, a row, or a group as a whole.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum Unit {
    Header(SharedString),
    Row(SharedString),
    Block(SharedString),
}

#[derive(Clone, PartialEq, Default)]
struct Shape {
    groups: Vec<GroupShape>,
    loose: Vec<SharedString>,
}

#[derive(Clone, PartialEq)]
struct GroupShape {
    key: SharedString,
    open: bool,
    rows: Vec<SharedString>,
}

/// The heights the last frame measured, which include the space below a row.
type Heights = HashMap<Unit, f32>;

struct Placed {
    unit: Unit,
    /// The group it is drawn in.
    group: Option<SharedString>,
    top: f32,
    height: f32,
}

struct Span {
    key: SharedString,
    top: f32,
    height: f32,
}

#[derive(Default)]
struct Layout {
    items: Vec<Placed>,
    blocks: Vec<Span>,
}

impl Layout {
    fn item(&self, unit: &Unit) -> Option<&Placed> {
        self.items.iter().find(|p| p.unit == *unit)
    }

    fn mid(&self, unit: &Unit) -> Option<f32> {
        self.item(unit).map(|p| p.top + p.height / 2.)
    }
}

fn height(heights: &Heights, unit: &Unit) -> f32 {
    heights.get(unit).copied().unwrap_or(0.)
}

impl Shape {
    fn layout(&self, heights: &Heights) -> Layout {
        let mut layout = Layout::default();
        let mut y = 0.;
        let put = |layout: &mut Layout, unit: Unit, group: Option<&SharedString>, y: &mut f32| {
            let h = height(heights, &unit);
            layout.items.push(Placed {
                unit,
                group: group.cloned(),
                top: *y,
                height: h,
            });
            *y += h;
        };
        for group in &self.groups {
            let top = y;
            put(
                &mut layout,
                Unit::Header(group.key.clone()),
                Some(&group.key),
                &mut y,
            );
            if group.open {
                for row in &group.rows {
                    put(
                        &mut layout,
                        Unit::Row(row.clone()),
                        Some(&group.key),
                        &mut y,
                    );
                }
            }
            layout.blocks.push(Span {
                key: group.key.clone(),
                top,
                height: y - top,
            });
        }
        for row in &self.loose {
            put(&mut layout, Unit::Row(row.clone()), None, &mut y);
        }
        layout
    }

    /// Whether the same rows are in the same groups and places, folds apart.
    fn same_structure(&self, other: &Shape) -> bool {
        self.loose == other.loose
            && self.groups.len() == other.groups.len()
            && self
                .groups
                .iter()
                .zip(&other.groups)
                .all(|(a, b)| a.key == b.key && a.rows == b.rows)
    }

    fn holds_row(&self, row: &str) -> bool {
        self.loose.iter().any(|r| r == row)
            || self.groups.iter().any(|g| g.rows.iter().any(|r| r == row))
    }

    fn holds_group(&self, key: &str) -> bool {
        self.groups.iter().any(|g| g.key == key)
    }

    /// Where `row` is now: its group and its index there.
    fn place_of(&self, row: &str) -> Option<(Option<SharedString>, usize)> {
        if let Some(at) = self.loose.iter().position(|r| r == row) {
            return Some((None, at));
        }
        self.groups.iter().find_map(|g| {
            g.rows
                .iter()
                .position(|r| r == row)
                .map(|at| (Some(g.key.clone()), at))
        })
    }

    /// The list with `row` put at `landing`.
    fn landed(&self, row: &SharedString, landing: &Landing) -> Shape {
        let mut shape = self.clone();
        shape.loose.retain(|r| r != row);
        shape
            .groups
            .iter_mut()
            .for_each(|g| g.rows.retain(|r| r != row));
        let rows = match &landing.group {
            Some(key) => shape
                .groups
                .iter_mut()
                .find(|g| g.key == *key)
                .map(|g| &mut g.rows),
            None => Some(&mut shape.loose),
        };
        if let Some(rows) = rows {
            rows.insert(landing.index.min(rows.len()), row.clone());
        }
        shape
    }
}

#[derive(Clone, PartialEq, Debug)]
struct Landing {
    group: Option<SharedString>,
    index: usize,
}

/// Where a row whose centre is at `c` lands: in the group whose block holds that height (the rows
/// below the last group are the rows outside any group), before the rows that have their middle
/// below it. A folded group has no rows drawn: the row goes to its end.
fn row_landing(shape: &Shape, layout: &Layout, row: &SharedString, c: f32) -> Landing {
    let others = |rows: &[SharedString]| -> Vec<SharedString> {
        rows.iter().filter(|r| *r != row).cloned().collect()
    };
    let before = |rows: &[SharedString]| {
        rows.iter()
            .filter(|r| {
                layout
                    .mid(&Unit::Row((*r).clone()))
                    .is_some_and(|mid| mid < c)
            })
            .count()
    };
    let zone = layout
        .blocks
        .iter()
        .position(|block| c < block.top + block.height);
    match zone {
        Some(at) => {
            let group = &shape.groups[at];
            let rows = others(&group.rows);
            let index = if group.open {
                before(&rows)
            } else {
                rows.len()
            };
            Landing {
                group: Some(group.key.clone()),
                index,
            }
        }
        None => Landing {
            group: None,
            index: before(&others(&shape.loose)),
        },
    }
}

enum Move {
    Row(RowMove),
    Group(SharedString, usize),
}

/// What a held unit asks of the list at a given height: how far each other unit goes to make
/// room for it, where it lands, and what that says.
struct Outcome {
    offsets: HashMap<Unit, f32>,
    /// The top and the height of the dashed slot, `None` when it lands out of sight.
    slot: Option<(f32, f32)>,
    /// The natural top of the held unit, which the slot starts from.
    from: f32,
    mv: Option<Move>,
    /// The group a held row goes into.
    into: Option<SharedString>,
}

fn outcome(shape: &Shape, heights: &Heights, gap: f32, held: &Unit, dy: f32) -> Option<Outcome> {
    let layout = shape.layout(heights);
    match held {
        Unit::Row(row) => {
            let natural = layout.item(held)?;
            let c = natural.top + dy + natural.height / 2.;
            let landing = row_landing(shape, &layout, row, c);
            let after = shape.landed(row, &landing).layout(heights);
            let offsets = layout
                .items
                .iter()
                .filter(|p| p.unit != *held)
                .filter_map(|p| after.item(&p.unit).map(|n| (p.unit.clone(), n.top - p.top)))
                .collect();
            let slot = after.item(held).map(|n| (n.top, (n.height - gap).max(0.)));
            let mv = (shape.place_of(row)? != (landing.group.clone(), landing.index)).then(|| {
                Move::Row(RowMove {
                    row: row.clone(),
                    group: landing.group.clone(),
                    index: landing.index,
                })
            });
            Some(Outcome {
                offsets,
                slot,
                from: natural.top,
                mv,
                into: landing.group,
            })
        }
        Unit::Block(key) => {
            let own = layout.blocks.iter().find(|b| b.key == *key)?;
            let c = own.top + dy + own.height / 2.;
            let mut order: Vec<&Span> = layout.blocks.iter().filter(|b| b.key != *key).collect();
            let index = order.iter().filter(|b| b.top + b.height / 2. < c).count();
            order.insert(index, own);
            let mut y = 0.;
            let mut offsets = HashMap::new();
            let mut slot = None;
            for span in order {
                if span.key == *key {
                    slot = Some((y, span.height));
                } else {
                    offsets.insert(Unit::Block(span.key.clone()), y - span.top);
                }
                y += span.height;
            }
            let was = shape.groups.iter().position(|g| g.key == *key)?;
            let mv = (was != index).then(|| Move::Group(key.clone(), index));
            Some(Outcome {
                offsets,
                slot,
                from: own.top,
                mv,
                into: None,
            })
        }
        Unit::Header(_) => None,
    }
}

/// What a group that came or went was: its heading, drawn again while it folds away.
#[derive(Clone)]
struct Memo {
    key: SharedString,
    title: SharedString,
    count: usize,
    open: bool,
    mark: Option<Build>,
    trailing: Option<Build>,
}

/// A group of the list or one that left and is folding away, in the order they are drawn.
struct Entry {
    memo: Memo,
    gone: Option<Instant>,
}

/// The first frame of a change the caller made: `new` as it is drawn then, with a group that left
/// still there (its heading alone), and without the rows that come, which the caller's own motion
/// brings in: only what stays is placed, so that what slides is what moved. A heading that comes
/// is not there yet (the caller gives its height as nothing).
fn first_frame(old: &Shape, new: &Shape, entries: &[Entry]) -> Shape {
    let kept = |rows: &[SharedString]| -> Vec<SharedString> {
        rows.iter().filter(|r| old.holds_row(r)).cloned().collect()
    };
    let groups = entries
        .iter()
        .filter_map(|entry| {
            let key = &entry.memo.key;
            if entry.gone.is_some() {
                return Some(GroupShape {
                    key: key.clone(),
                    open: false,
                    rows: Vec::new(),
                });
            }
            let group = new.groups.iter().find(|g| g.key == *key)?;
            Some(GroupShape {
                key: key.clone(),
                open: group.open,
                rows: kept(&group.rows),
            })
        })
        .collect();
    Shape {
        groups,
        loose: kept(&new.loose),
    }
}

/// Where each unit starts from, as how far it is from its place, when the caller changed the list
/// from outside: so that it is drawn where it was and slides to where it goes. `now` is the
/// offsets it has now, which a slide still running goes on from.
fn carried(
    old: &Shape,
    new: &Shape,
    heights: &Heights,
    now: &dyn Fn(&Unit) -> f32,
) -> HashMap<Unit, f32> {
    let was = old.layout(heights);
    let will = new.layout(heights);
    let mut starts: HashMap<Unit, f32> = HashMap::new();
    for block in &will.blocks {
        if let Some(prev) = was.blocks.iter().find(|b| b.key == block.key) {
            let unit = Unit::Block(block.key.clone());
            starts.insert(unit.clone(), prev.top + now(&unit) - block.top);
        }
    }
    let mut items = HashMap::new();
    for item in &will.items {
        let Some(prev) = was.item(&item.unit) else {
            continue;
        };
        let of = |group: &Option<SharedString>, starts: &HashMap<Unit, f32>| {
            group.as_ref().map_or(0., |g| {
                let unit = Unit::Block(g.clone());
                starts.get(&unit).copied().unwrap_or_else(|| now(&unit))
            })
        };
        let from = prev.top + now(&item.unit) + group_offset(&prev.group, now);
        items.insert(
            item.unit.clone(),
            from - item.top - of(&item.group, &starts),
        );
    }
    starts.extend(items);
    starts.retain(|unit, start| (*start - now(unit)).abs() > 0.5);
    starts
}

fn group_offset(group: &Option<SharedString>, now: &dyn Fn(&Unit) -> f32) -> f32 {
    group.as_ref().map_or(0., |g| now(&Unit::Block(g.clone())))
}

// ── What the list remembers between frames ─────────────────────────────────────────────────────

#[derive(Clone)]
struct Drag {
    unit: Unit,
    start: Point<Pixels>,
    pos: Point<Pixels>,
}

/// A drop that asked for a move: how far each unit was to go, until the next frame says
/// whether the caller applied it.
struct Pending {
    before: Shape,
    offsets: HashMap<Unit, f32>,
}

#[derive(Clone, Default)]
struct Callbacks {
    move_row: Option<OnMoveRow>,
    move_group: Option<OnMoveGroup>,
}

struct State {
    focus: FocusHandle,
    /// Whether a frame was drawn: what is there then is simply there, what comes later unfolds.
    started: bool,
    /// The groups drawn last, and those that left and are still folding away.
    entries: Vec<Entry>,
    shape: Shape,
    gap: f32,
    callbacks: Callbacks,
    heights: Rc<RefCell<Heights>>,
    frame: Rc<Cell<Bounds<Pixels>>>,
    candidate: Option<(Unit, Point<Pixels>)>,
    drag: Option<Drag>,
    /// How far each unit is from its place now: what a slide or a settle moves.
    motions: HashMap<Unit, Motion>,
    pending: Option<Pending>,
    /// The unit that was held last, drawn lifted until it has settled, and how far.
    lifted: Option<Unit>,
    lift: Motion,
    slot_top: Motion,
    slot_height: f32,
    slot_fade: Motion,
}

/// What a frame draws.
struct Plan {
    /// The groups in the order they are drawn, with those that left folding away.
    groups: Vec<(Memo, bool)>,
    /// The groups that came since the last frame.
    entering: HashSet<SharedString>,
    offsets: HashMap<Unit, f32>,
    held: Option<(Unit, f32)>,
    lifted: Option<Unit>,
    lift: f32,
    slot: Option<(f32, f32, f32)>,
    into: Option<SharedString>,
    dragging: bool,
}

impl Plan {
    fn offset(&self, unit: &Unit) -> f32 {
        match &self.held {
            Some((held, dy)) if held == unit => *dy,
            _ => self.offsets.get(unit).copied().unwrap_or(0.),
        }
    }

    fn is_lifted(&self, unit: &Unit) -> bool {
        self.lifted.as_ref() == Some(unit)
    }
}

impl State {
    fn new(focus: FocusHandle) -> Self {
        Self {
            focus,
            started: false,
            entries: Vec::new(),
            shape: Shape::default(),
            gap: 0.,
            callbacks: Callbacks::default(),
            heights: Rc::new(RefCell::new(HashMap::new())),
            frame: Rc::new(Cell::new(Bounds::default())),
            candidate: None,
            drag: None,
            motions: HashMap::new(),
            pending: None,
            lifted: None,
            lift: Motion::settled(0.),
            slot_top: Motion::settled(0.),
            slot_height: 0.,
            slot_fade: Motion::settled(0.),
        }
    }

    fn dy(drag: &Drag) -> f32 {
        f32::from(drag.pos.y - drag.start.y)
    }

    /// How far the group a held row is in is from its place: the row is drawn in it, and is held
    /// at the pointer all the same.
    fn group_offset(&self, unit: &Unit, now: Instant) -> f32 {
        let Unit::Row(row) = unit else { return 0. };
        let Some((Some(group), _)) = self.shape.place_of(row) else {
            return 0.;
        };
        self.motions
            .get(&Unit::Block(group))
            .map_or(0., |m| m.value(now))
    }

    fn outcome(&self) -> Option<Outcome> {
        let drag = self.drag.as_ref()?;
        outcome(
            &self.shape,
            &self.heights.borrow(),
            self.gap,
            &drag.unit,
            Self::dy(drag),
        )
    }

    /// The drag is over: what was held slides from where it is to its place, and the slot fades.
    fn let_go(&mut self, now: Instant, reduce: bool) {
        if let Some(drag) = self.drag.take() {
            let from = Self::dy(&drag) - self.group_offset(&drag.unit, now);
            self.motions
                .insert(drag.unit.clone(), Motion::settled(from));
        }
        self.lift.animate(0., PANEL, now, reduce);
        self.slot_fade.animate(0., PANEL, now, reduce);
    }

    fn start(
        &mut self,
        unit: Unit,
        start: Point<Pixels>,
        pos: Point<Pixels>,
        now: Instant,
        reduce: bool,
    ) {
        self.drag = Some(Drag {
            unit: unit.clone(),
            start,
            pos,
        });
        self.motions.remove(&unit);
        self.lifted = Some(unit);
        self.lift.animate(1., HOVER, now, reduce);
        if let Some(out) = self.outcome() {
            self.slot_top = Motion::settled(out.from);
            self.slot_height = out.slot.map_or(0., |(_, h)| h);
        }
        self.slot_fade.animate(1., HOVER, now, reduce);
    }

    /// The groups now, in order, and those that came: a group that left stays where it was while it
    /// folds away (over [`PANEL`]), after the nearest group before it that stayed.
    fn groups_now(
        &mut self,
        memos: Vec<Memo>,
        now: Instant,
        reduce: bool,
    ) -> (Vec<Entry>, HashSet<SharedString>) {
        let mut old = std::mem::take(&mut self.entries);
        old.retain(|e| {
            e.gone
                .is_none_or(|at| !reduce && now.saturating_duration_since(at) < PANEL)
        });
        let entering = memos
            .iter()
            .filter(|m| self.started && !old.iter().any(|e| e.memo.key == m.key))
            .map(|m| m.key.clone())
            .collect();
        let mut after: HashMap<Option<SharedString>, Vec<Entry>> = HashMap::new();
        let mut before = None;
        for mut entry in old {
            if memos.iter().any(|m| m.key == entry.memo.key) {
                before = Some(entry.memo.key.clone());
                continue;
            }
            if entry.gone.is_none() {
                if reduce {
                    continue;
                }
                entry.gone = Some(now);
            }
            after.entry(before.clone()).or_default().push(entry);
        }
        let mut entries = after.remove(&None).unwrap_or_default();
        for memo in memos {
            let key = Some(memo.key.clone());
            entries.push(Entry { memo, gone: None });
            entries.extend(after.remove(&key).unwrap_or_default());
        }
        (entries, entering)
    }

    /// The frame to draw: moves what moves toward where it goes, and says whether it still runs.
    fn plan(
        &mut self,
        shape: Shape,
        memos: Vec<Memo>,
        gap: f32,
        callbacks: Callbacks,
        now: Instant,
        reduce: bool,
    ) -> (Plan, bool) {
        let accepted = self.pending.as_ref().map(|p| p.before != shape);
        let dropped = self.pending.is_some();
        let old = std::mem::replace(&mut self.shape, shape);
        self.gap = gap;
        self.callbacks = callbacks;
        // What is held is gone (the caller removed it): there is nothing left to follow.
        if let Some(drag) = &self.drag {
            let there = match &drag.unit {
                Unit::Row(row) => self.shape.holds_row(row),
                Unit::Block(key) => self.shape.holds_group(key),
                Unit::Header(_) => false,
            };
            if !there {
                self.let_go(now, reduce);
            }
        }
        let (entries, entering) = self.groups_now(memos, now, reduce);
        self.entries = entries;
        // The caller changed the list from outside, not by a drop: what moved starts where it was.
        // A change while a row is held is one too: the rest slides, and what is held stays where
        // the pointer has it.
        let outside = self.started && !dropped && !old.same_structure(&self.shape);
        if outside {
            let first = first_frame(&old, &self.shape, &self.entries);
            let value = |unit: &Unit| self.motions.get(unit).map_or(0., |m| m.value(now));
            let mut heights = self.heights.borrow().clone();
            for key in &entering {
                heights.insert(Unit::Header(key.clone()), 0.);
            }
            let mut starts = carried(&old, &first, &heights, &value);
            if let Some(drag) = &mut self.drag {
                // What is held is drawn at the pointer, wherever its place went: the pointer's
                // origin moves by how far its natural place did. Its start says where it is drawn
                // (that place with the slide of its group still to go), and the held row is drawn
                // without that slide, so the group's slide it has now is taken off.
                let own = starts.remove(&drag.unit).unwrap_or(0.);
                let group = |shape: &Shape| match &drag.unit {
                    Unit::Row(row) => shape.place_of(row).and_then(|(group, _)| group),
                    _ => None,
                };
                let block = group(&self.shape).map_or(0., |g| {
                    let unit = Unit::Block(g);
                    starts.get(&unit).copied().unwrap_or_else(|| value(&unit))
                });
                let slide = group(&old).map_or(0., |g| value(&Unit::Block(g)));
                drag.start.y -= px(own + block - slide);
            }
            for (unit, start) in starts {
                self.motions.insert(unit, Motion::settled(start));
            }
        }
        self.started = true;
        if let Some(pending) = self.pending.take() {
            // The caller applied the move: every unit is now in the place it was going to, so
            // what is left to travel is what it still was from there. Refused, it goes back from
            // where it is.
            if accepted == Some(true) {
                for (unit, motion) in &mut self.motions {
                    let gone = pending.offsets.get(unit).copied().unwrap_or(0.);
                    *motion = Motion::settled(motion.value(now) - gone);
                }
            }
        }

        let out = self.outcome();
        let mut targets: HashMap<Unit, f32> = HashMap::new();
        if let Some(out) = &out {
            targets = out.offsets.clone();
            if let Some((top, h)) = out.slot {
                if self.slot_top.target() != top {
                    self.slot_top.animate(top, PANEL, now, reduce);
                }
                self.slot_height = h;
            }
            // A row that lands in a folded group lands out of sight: no slot to show.
            let want = if out.slot.is_some() { 1. } else { 0. };
            if self.slot_fade.target() != want {
                self.slot_fade.animate(want, HOVER, now, reduce);
            }
        }
        for unit in self.motions.keys().cloned().collect::<Vec<_>>() {
            targets.entry(unit).or_insert(0.);
        }
        for (unit, target) in &targets {
            let motion = self
                .motions
                .entry(unit.clone())
                .or_insert(Motion::settled(0.));
            if motion.target() != *target {
                motion.animate(*target, PANEL, now, reduce);
            }
        }

        let leaving = self.entries.iter().any(|e| e.gone.is_some());
        let running = leaving
            || self.motions.values().any(|m| m.is_running(now))
            || self.lift.is_running(now)
            || self.slot_top.is_running(now)
            || self.slot_fade.is_running(now);
        if !running {
            self.motions.retain(|_, m| m.target() != 0.);
            self.motions.values_mut().for_each(|m| m.settle(now));
            self.slot_top.settle(now);
            self.lift.settle(now);
            self.slot_fade.settle(now);
            if self.drag.is_none() {
                self.lifted = None;
            }
        }

        let offsets: HashMap<Unit, f32> = self
            .motions
            .iter()
            .map(|(unit, motion)| (unit.clone(), motion.value(now)))
            .collect();
        let slot_fade = self.slot_fade.value(now);
        let slot =
            (slot_fade > 0.).then(|| (self.slot_top.value(now), self.slot_height, slot_fade));
        let plan = Plan {
            groups: self
                .entries
                .iter()
                .map(|e| (e.memo.clone(), e.gone.is_some()))
                .collect(),
            entering,
            offsets,
            held: self.drag.as_ref().map(|d| {
                (
                    d.unit.clone(),
                    Self::dy(d) - self.group_offset(&d.unit, now),
                )
            }),
            lifted: self.lifted.clone(),
            lift: self.lift.value(now),
            slot,
            into: out.and_then(|o| o.into),
            dragging: self.drag.is_some(),
        };
        (plan, running)
    }
}

// ── The pointer ────────────────────────────────────────────────────────────────────────────────

fn press(state: &Entity<State>, unit: Unit, at: Point<Pixels>, cx: &mut App) {
    state.update(cx, |s, _| s.candidate = Some((unit, at)));
}

/// Follows the pointer: a press that went past the threshold becomes a drag.
fn moved(state: &Entity<State>, at: Point<Pixels>, held: bool, window: &mut Window, cx: &mut App) {
    let now = cx.background_executor().now();
    let reduce = cx.reduce_motion();
    let focus = state.update(cx, |s, cx| {
        if !held {
            // The button came up out of reach (outside the window): the drag is over.
            s.candidate = None;
            if s.drag.is_some() {
                s.let_go(now, reduce);
                cx.notify();
            }
            return None;
        }
        if let Some(drag) = &mut s.drag {
            drag.pos = at;
            cx.notify();
            return None;
        }
        let (unit, start) = s.candidate.clone()?;
        if (at - start).magnitude() <= THRESHOLD {
            return None;
        }
        s.candidate = None;
        s.start(unit, start, at, now, reduce);
        cx.notify();
        Some(s.focus.clone())
    });
    if let Some(focus) = focus {
        window.focus(&focus, cx);
    }
}

/// The button comes up: a drop in another place says what moved; otherwise everything goes back.
fn released(state: &Entity<State>, at: Point<Pixels>, window: &mut Window, cx: &mut App) {
    let now = cx.background_executor().now();
    let reduce = cx.reduce_motion();
    let said = state.update(cx, |s, cx| {
        s.candidate = None;
        s.drag.as_ref()?;
        let frame = s.frame.get();
        let outside = at.x < frame.left() || at.x > frame.right();
        let out = if outside { None } else { s.outcome() };
        let callbacks = s.callbacks.clone();
        let before = s.shape.clone();
        let held = s.drag.as_ref().map(|d| d.unit.clone());
        s.let_go(now, reduce);
        cx.notify();
        let mut out = out?;
        let mv = out.mv?;
        // What was held goes to the slot like the others go to theirs.
        if let (Some(held), Some((top, _))) = (held, out.slot) {
            out.offsets.insert(held, top - out.from);
        }
        s.pending = Some(Pending {
            before,
            offsets: out.offsets,
        });
        Some((mv, callbacks))
    });
    match said {
        Some((Move::Row(mv), callbacks)) => {
            if let Some(f) = callbacks.move_row {
                f(&mv, window, cx);
            }
        }
        Some((Move::Group(key, index), callbacks)) => {
            if let Some(f) = callbacks.move_group {
                f(&key, index, window, cx);
            }
        }
        None => {}
    }
}

fn cancel(state: &Entity<State>, cx: &mut App) -> bool {
    let now = cx.background_executor().now();
    let reduce = cx.reduce_motion();
    state.update(cx, |s, cx| {
        let was = s.drag.is_some();
        s.candidate = None;
        s.let_go(now, reduce);
        cx.notify();
        was
    })
}

// ── Drawing ────────────────────────────────────────────────────────────────────────────────────

/// Reports the height of the element it lies in, which a drag needs to know where the others go.
fn measure(unit: Unit, heights: &Rc<RefCell<Heights>>) -> AnyElement {
    let heights = heights.clone();
    canvas(
        move |bounds: Bounds<Pixels>, _, _| {
            heights
                .borrow_mut()
                .insert(unit, f32::from(bounds.size.height));
        },
        |_, _, _, _| {},
    )
    .absolute()
    .size_full()
    .into_any_element()
}

/// Draws `content` where the plan puts the unit; the one held is lifted and drawn over the rest.
fn placed(id: ElementId, unit: &Unit, plan: &Plan, content: AnyElement, cx: &App) -> AnyElement {
    let offset = plan.offset(unit);
    let mut frame = div().id(id).w_full().relative().top(px(offset));
    let lifted = plan.is_lifted(unit);
    if lifted {
        let p = palette(cx);
        frame = frame.bg(p.card.opacity(plan.lift)).drop_shadow(
            vec![BoxShadow {
                color: p.shadow.opacity(0.5 * plan.lift),
                offset: point(px(0.), px(6.)),
                blur_radius: px(16.),
                spread_radius: px(0.),
                inset: false,
            }],
            &p,
        );
    }
    let frame = frame.child(content).test_support();
    if lifted {
        deferred(frame).into_any_element()
    } else {
        frame.into_any_element()
    }
}

impl RenderOnce for Reorderable {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Reorderable {
            id,
            gap,
            groups,
            rows,
            on_move_row,
            on_move_group,
            on_toggle,
        } = self;
        let shape = Shape {
            groups: groups
                .iter()
                .map(|g| GroupShape {
                    key: g.key.clone(),
                    open: g.open,
                    rows: g.rows.iter().map(|(key, _)| key.clone()).collect(),
                })
                .collect(),
            loose: rows.iter().map(|(key, _)| key.clone()).collect(),
        };
        let state =
            window.use_keyed_state(part(&id, "reorder"), cx, |_, cx: &mut Context<State>| {
                State::new(cx.focus_handle())
            });
        let now = cx.background_executor().now();
        let reduce = cx.reduce_motion();
        let memos = groups
            .iter()
            .map(|g| Memo {
                key: g.key.clone(),
                title: g.title.clone(),
                count: g.rows.len(),
                open: g.open,
                mark: g.mark.clone(),
                trailing: g.trailing.clone(),
            })
            .collect();
        let (plan, running) = state.update(cx, |s, _| {
            s.plan(
                shape,
                memos,
                gap,
                Callbacks {
                    move_row: on_move_row,
                    move_group: on_move_group,
                },
                now,
                reduce,
            )
        });
        if running {
            window.request_animation_frame();
        }
        let (heights, frame, focus) = {
            let s = state.read(cx);
            (s.heights.clone(), s.frame.clone(), s.focus.clone())
        };
        let p = palette(cx);

        let row_frame = |key: SharedString, content: AnyElement, cx: &mut App| {
            let unit = Unit::Row(key.clone());
            let press_state = state.clone();
            let press_unit = unit.clone();
            let content = div()
                .w_full()
                .pb(px(gap))
                .relative()
                .on_mouse_down(MouseButton::Left, move |down, _, cx| {
                    press(&press_state, press_unit.clone(), down.position, cx)
                })
                .child(content)
                .child(measure(unit.clone(), &heights))
                .into_any_element();
            placed(part(&id, &format!("row-{key}")), &unit, &plan, content, cx)
        };

        let mut column = div()
            .id(id.clone())
            .w_full()
            .relative()
            .flex()
            .flex_col()
            .test_support();
        if plan.dragging {
            // Escape reaches the list while it holds the focus, which it takes with the drag: a
            // list that tracked it all the time would take the focus on every click on a row.
            let state = state.clone();
            column = column.track_focus(&focus).on_key_down(move |event, _, cx| {
                if event.keystroke.key == "escape" && cancel(&state, cx) {
                    cx.stop_propagation();
                }
            });
        }

        let mut live: HashMap<SharedString, ReorderGroup> =
            groups.into_iter().map(|g| (g.key.clone(), g)).collect();
        for (memo, leaving) in &plan.groups {
            let Memo {
                key,
                title,
                count,
                open,
                mark,
                trailing,
            } = memo.clone();
            // A group that left has its heading alone, folding away, as it was drawn last.
            let group = live.remove(&key);
            let gid = part(&id, &format!("group-{key}"));
            let block_unit = Unit::Block(key.clone());
            let header_unit = Unit::Header(key.clone());
            let hid = part(&gid, "header");

            let toggle = on_toggle.clone();
            let toggled = key.clone();
            let drag_state = state.clone();
            let drag_unit = block_unit.clone();
            let mut header = div()
                .id(hid.clone())
                .aria_expanded(open)
                .w_full()
                .px(space::SM)
                .pt(space::SM)
                .pb(space::XS)
                .flex()
                .items_center()
                .gap(space::SM)
                .cursor_pointer();
            if !leaving {
                header = header
                    .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                        press(&drag_state, drag_unit.clone(), event.position, cx)
                    })
                    .on_click(move |_, window, cx| {
                        if let Some(toggle) = &toggle {
                            toggle(&toggled, !open, window, cx);
                        }
                    });
            }
            let mut header = header.child(chevron(part(&gid, "chevron"), open, window, cx));
            let rows = group.map(|g| g.rows).unwrap_or_default();
            if let Some(mark) = mark {
                let mark = mark(window, cx);
                header =
                    header.children(unfolding_mark(part(&gid, "mark"), mark, !open, window, cx));
            }
            let mut header = header
                .child(
                    div()
                        .id(part(&gid, "title"))
                        .aria_label(title.clone())
                        .flex_none()
                        .mono(11.)
                        .font_weight(FontWeight::MEDIUM)
                        .whitespace_nowrap()
                        .child(title.to_uppercase())
                        .test_support(),
                )
                .child(
                    div()
                        .id(part(&gid, "count"))
                        .aria_label(SharedString::from(count.to_string()))
                        .flex_none()
                        .mono(11.)
                        .child(count.to_string())
                        .test_support(),
                );
            if let Some(trailing) = trailing {
                let trailing = trailing(window, cx);
                // Its own press: the heading's drag and fold are not its business.
                let mut slot = div()
                    .id(part(&gid, "trailing"))
                    .flex_none()
                    .ml_auto()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation());
                if *leaving {
                    // What is gone is not pressed: the button of its heading takes nothing while
                    // it folds away, its press is stopped before it reaches it.
                    slot = slot
                        .capture_any_mouse_down(|_, _, cx| cx.stop_propagation())
                        .capture_any_mouse_up(|_, _, cx| cx.stop_propagation());
                }
                header = header.child(slot.child(trailing).test_support());
            }
            // The group a held row would go into says so.
            let into = fade(
                part(&hid, "into"),
                plan.into.as_ref() == Some(&key) && plan.dragging,
                HOVER,
                window,
                cx,
            );
            let header_id = hid.clone();
            let header = hoverable(header, move |d, t| {
                seen::note(&header_id, t);
                d.bg(p.hover.opacity(into))
                    .text_color(mix(p.faint, p.text, t))
            });
            let head_frame = div()
                .w_full()
                .relative()
                .child(header)
                .child(measure(header_unit.clone(), &heights));
            let head = placed(
                part(&gid, "head"),
                &header_unit,
                &plan,
                head_frame.into_any_element(),
                cx,
            );

            // The heading unfolds as it comes and folds away as it leaves, what is below with it.
            let head = presence(part(&gid, "presence"), !leaving, head)
                .entering(plan.entering.contains(&key));
            let rows: Vec<AnyElement> = rows
                .into_iter()
                .map(|(row, content)| row_frame(row, content, cx))
                .collect();
            let body = presence(
                part(&gid, "body"),
                open,
                div().w_full().flex().flex_col().children(rows),
            );
            let block = div()
                .w_full()
                .flex()
                .flex_col()
                .child(head)
                .child(body)
                .into_any_element();
            column = column.child(placed(gid, &block_unit, &plan, block, cx));
        }
        for (key, content) in rows {
            column = column.child(row_frame(key, content, cx));
        }

        if let Some((top, h, t)) = plan.slot {
            column = column.child(
                div()
                    .id(part(&id, "slot"))
                    .absolute()
                    .top(px(top))
                    .left_0()
                    .w_full()
                    .h(px(h))
                    .surface(Surface::new(px(8.)).line(p.line2.opacity(t)).dashed())
                    .test_support(),
            );
        }
        if plan.dragging {
            // The whole window, so that the row that is held, drawn wherever the pointer is, is not
            // what the button comes up on. Above that row too, which is drawn over the others.
            let origin = frame.get().origin;
            column = column.child(
                deferred(
                    div()
                        .id(part(&id, "sheet"))
                        .absolute()
                        .top(-origin.y)
                        .left(-origin.x)
                        .w(window.viewport_size().width)
                        .h(window.viewport_size().height)
                        .occlude()
                        .cursor(CursorStyle::ClosedHand)
                        .test_support(),
                )
                .with_priority(1),
            );
        }

        // The pointer is followed at the window's level: the list is only where it started.
        let moving = state.clone();
        let upping = state.clone();
        column.child(
            canvas(
                move |bounds: Bounds<Pixels>, _, _| frame.set(bounds),
                move |_, _, window, _| {
                    let moving = moving.clone();
                    window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                        if phase == DispatchPhase::Capture {
                            moved(
                                &moving,
                                event.position,
                                event.pressed_button == Some(MouseButton::Left),
                                window,
                                cx,
                            );
                        }
                    });
                    let upping = upping.clone();
                    window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                        if phase == DispatchPhase::Capture && event.button == MouseButton::Left {
                            released(&upping, event.position, window, cx);
                        }
                    });
                },
            )
            .absolute()
            .size_full(),
        )
    }
}

/// The mark a folded group's header shows: it unfolds sideways and fades in as the group folds,
/// and goes the same way when it opens. Nothing while the group is open and the mark gone.
fn unfolding_mark(
    id: ElementId,
    mark: AnyElement,
    folded: bool,
    window: &mut Window,
    cx: &mut App,
) -> Option<AnyElement> {
    let key = ElementId::NamedChild(Arc::new(id.clone()), "shown".into());
    let state = window.use_keyed_state(key, cx, move |_, _| {
        Shown::new(Motion::settled(if folded { 1. } else { 0. }))
    });
    let at = drive(&state, folded, window, cx);
    if !folded && !at.running {
        return None;
    }
    let width = at.width.clone();
    let measured = canvas(
        move |bounds: Bounds<Pixels>, _, _| width.set(bounds.size.width),
        |_, _, _, _| {},
    )
    .absolute()
    .size_full();
    let natural = div()
        .flex_none()
        .relative()
        .flex()
        .items_center()
        .child(mark)
        .child(measured);
    let mut frame = div().id(id).flex_none().overflow_hidden().opacity(at.t);
    if at.running {
        frame = frame.w(at.width.get() * at.t);
    }
    Some(frame.child(natural).test_support().into_any_element())
}
