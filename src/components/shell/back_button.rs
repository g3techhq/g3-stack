use crate::app::Route;
use dioxus::prelude::*;
use g3_route_transitions::animated_back_or_navigate;

/// Back for pushed pages and sheets, meant for `Header`'s `start` slot.
///
/// g3-ui's `BackButton` draws it; this supplies the behaviour, since only the
/// app knows its `Route`. `animated_back_or_navigate` pops real history with
/// the reverse of the route's own animation (a sheet dismisses downward, a
/// pushed page slides back). The fallback is only reached when there is no
/// history to pop — someone opened a link straight to this page, or reloaded
/// on it — which is exactly when a hardcoded "go home" would strand them
/// somewhere unrelated to where they are.
#[component]
pub fn BackButton() -> Element {
    let route: Route = use_route();
    let fallback = match route {
        // Editing a note belongs to that note; leaving the editor should land
        // on it, not back at the list.
        Route::EditNote { id } => Route::NoteDetail { id },
        _ => Route::Notes { filter: None },
    };

    rsx! {
        g3_ui::BackButton {
            onclick: move |_| {
                spawn(animated_back_or_navigate(fallback.clone()));
            },
        }
    }
}
