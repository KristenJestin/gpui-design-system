//! The shared window components of an application: tokens, rows, cards, menus, dialogs, toasts and
//! the motion they share.
//!
//! Every screen is built from these pieces, with the same tokens, the same states and the same
//! motion. A second, slightly different version of one of them is a defect. The kit knows nothing
//! of what an application is about: the application tells it its name and its look at [`init`].

pub mod appearance;
pub mod bulk_bar;
pub mod button;
pub mod card;
pub mod checkbox;
pub mod chip_picker;
pub mod choice_row;
pub mod color;
pub mod confirm;
pub mod cut;
pub mod diff;
pub mod disclosure;
pub mod drawer;
pub mod dropdown;
pub mod field;
pub mod filter_chips;
pub mod fuzzy;
pub mod group;
pub mod icons;
pub mod identity;
mod ids;
mod interact;
pub mod layer;
pub mod lines;
pub mod listing;
pub mod look;
pub mod menu;
pub mod modal;
pub mod model_choice;
pub mod motion;
#[cfg(feature = "omarchy")]
pub mod omarchy;
pub mod page;
pub mod pager;
pub mod path;
pub mod progress;
pub mod qr_code;
pub mod radio;
pub mod reveal;
pub mod rows;
pub mod search_field;
pub mod seen;
pub mod segmented;
pub mod select;
pub mod set_here;
pub mod skeleton;
pub mod space;
pub mod subpage;
pub mod swatches;
pub mod switch;
pub mod tabs;
pub mod tag;
pub mod theme;
pub mod toast;
pub mod token_field;
pub mod typography;
pub mod unfolding;

use gpui_kit::App;

pub use appearance::{Appearance, DesktopTheme, Source};
pub use bulk_bar::BulkBar;
pub use button::{Badge, Button, ButtonKind, Tooltip, tipped};
pub use card::{Block, Card};
pub use checkbox::{CheckState, Checkbox};
pub use chip_picker::{ChipOption, ChipPicker, RemovableChip};
pub use choice_row::ChoiceRow;
pub use confirm::Confirm;
pub use cut::{QUOTED_CHARS, quoted, quoted_title, tipped_when_cut};
pub use diff::{DiffLine, DiffLines};
pub use disclosure::Disclosure;
pub use drawer::{Drawer, Side};
pub use dropdown::{Align, Dropdown, MenuItem};
pub use field::{Field, StyledField, field_style, follow_theme};
pub use filter_chips::{FilterChip, FilterChips};
pub use group::{Group, Grouped};
pub use icons::{Icon, Icons, icon};
pub use identity::{Identity, identity};
pub use interact::{
    Hoverable, dismiss_on_navigate, dismiss_top_overlay, fade, forget_overlays, hoverable,
    keys_held, open_overlay, pressable, selectable, text_region, tint,
};
pub use lines::{
    CardHead, Chip, FlatRow, FooterAction, LINE_GAP, LabelledLine, caption, lines_block,
};
pub use listing::{Item, Listing, Placement};
pub use look::{Corners, Fonts, Look, Shadow, Sizes, look};
pub use menu::Menu;
pub use modal::Modal;
pub use model_choice::{Chosen, ModelChoice, ModelEntry};
pub use page::{EmptyLine, PageHeader, Section};
pub use pager::{Dots, Sliding, sliding};
pub use path::path_line;
pub use progress::Progress;
pub use qr_code::QrCode;
pub use radio::RadioChoices;
pub use reveal::{
    Changing, Fold, Growing, Presence, PresenceOf, PulsingLabel, changed, changing, enter,
    fold_frame, growing, presence, presence_of,
};
pub use rows::{
    ListCard, ListRow, Tie, TreeRow, meta, section_label, tie_below_id, tie_id, tie_through_id,
    tie_turn_id, toggle_id, tree_children, tree_tie, turning,
};
pub use search_field::SearchField;
pub use segmented::Segmented;
pub use select::{Select, SelectOption, Tone};
pub use set_here::SetHere;
pub use skeleton::{Awaiting, Skeleton, awaiting};
pub use subpage::Subpage;
pub use swatches::{Swatch, Swatches};
pub use switch::Switch;
pub use tabs::Tabs;
pub use tag::Tag;
pub use theme::{Palette, Theme, palette};
pub use toast::{Toast, ToastKind, ToastStack, toast};
pub use token_field::{Piece, Token, TokenEvent, TokenField, TokenLook, TokenText};
pub use typography::Type;
pub use unfolding::{Summary, Unfolding};

/// Initialises gpui-kit (which reads the system's reduced-motion setting), the bundled fonts and
/// the theme, for the application `identity`: its first desktop source's when it offers one and
/// it can be used, its first theme's otherwise. Register [`Icons`], or assets that serve it, as
/// the application's assets. The kit is drawn in its own look ([`Look::default`]).
pub fn init(identity: Identity, cx: &mut App) {
    init_with(identity, Look::default(), cx);
}

/// [`init`], the kit drawn in `look`: the application's faces, corners, borders, shadow and
/// sizes. Register the files of its own faces with [`typography::add_fonts`].
pub fn init_with(identity: Identity, look: Look, cx: &mut App) {
    gpui_kit::init(cx);
    cx.set_global(identity);
    typography::register(cx);
    look::install(&look);
    appearance::install(cx);
}
