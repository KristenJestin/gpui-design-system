//! The ids of a component's parts: `<id>-chevron` for a name, a child of the id otherwise.

use std::sync::Arc;

use gpui_kit::ElementId;

/// The id of the part `name` of the component `id`, which a test finds as `<id>-<name>` when the
/// id is a name.
pub(crate) fn part(id: &ElementId, name: &str) -> ElementId {
    match id {
        ElementId::Name(text) => ElementId::Name(format!("{text}-{name}").into()),
        other => ElementId::NamedChild(Arc::new(other.clone()), name.to_owned().into()),
    }
}
