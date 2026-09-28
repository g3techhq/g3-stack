use dioxus::prelude::*;
use dioxus_icons::lucide::CircleAlert;
use g3_ui::{Color, EmptyState};

/// What a screen shows in place of data it could not load. The server's own
/// error text is for logs, not for someone looking at their notes.
#[component]
pub fn LoadFailed() -> Element {
    rsx! {
        EmptyState {
            title: "Couldn't load this",
            color: Color::Danger,
            icon: rsx! {
                CircleAlert { size: 40 }
            },
            "Check your connection and try again."
        }
    }
}

/// Something from row `index` of a cached list, read without subscribing.
///
/// For a row's event handler: the row captures only its index (a `usize`,
/// Copy) and looks up the id it needs when tapped, instead of cloning that
/// id into a closure on every render.
pub fn peek_row<T: 'static, R>(
    cached: &g3_cache::Cached<Vec<T>>,
    index: usize,
    pick: impl FnOnce(&T) -> R,
) -> Option<R> {
    cached
        .peek()
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .and_then(|rows| rows.get(index))
        .map(pick)
}
