//! How far a component is in a state that has no other trace, for the kit's own tests: a chevron
//! half turned, a summary half faded, a hover half in. Nothing in the window says it (GPUI
//! exposes no opacity, no rotation), and an accessibility value would be read aloud as the
//! element's own. A component notes it when it draws; a test reads it, on its own thread.

use std::cell::RefCell;
use std::collections::HashMap;

use gpui_kit::ElementId;

thread_local! {
    static SEEN: RefCell<HashMap<ElementId, f32>> = RefCell::new(HashMap::new());
}

/// Records that `id` was drawn `value` (0 to 1) along its state.
pub(crate) fn note(id: &ElementId, value: f32) {
    SEEN.with(|seen| seen.borrow_mut().insert(id.clone(), value));
}

/// What `id` was last drawn at, `None` if it never was on this thread.
#[doc(hidden)]
pub fn seen(id: impl Into<ElementId>) -> Option<f32> {
    SEEN.with(|seen| seen.borrow().get(&id.into()).copied())
}
