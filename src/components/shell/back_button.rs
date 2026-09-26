use crate::app::Route;
use dioxus::prelude::*;
use dioxus_icons::lucide::ChevronLeft;
use g3_ui::{Button, ButtonStyle};

/// Back chevron for `Header`'s `start_button` slot.
///
/// `animated_go_back` takes the outgoing snapshot, derives the reverse
/// animation from the route's own metadata (a sheet dismisses downward, a
/// pushed page slides right), and *then* pops real browser history. The
/// fallback below is only reached when there is no history to pop — someone
/// opened a deep link straight to this page, or refreshed on it — which is
/// exactly when a hardcoded "go to the home route" would strand them
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
        Button {
            style: ButtonStyle::Clear,
            aria_label: Some("Back".to_string()),
            onclick: move |_| {
                let fallback = fallback.clone();
                spawn(async move {
                    g3_route_transitions::animated_go_back(fallback).await;
                });
            },
            ChevronLeft { size: 20 }
        }
    }
}
