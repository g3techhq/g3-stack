use crate::{app::Route, auth::sign_out, state::AppState};
use dioxus::prelude::*;
use g3_ui::{ConfirmModal, Toast, ToastPosition};

/// Overlays that belong to the app rather than to any one screen.
///
/// Mounted once in `RootLayout`, above every route: their backdrops are not
/// clipped by whatever scroll container the current screen has, and a toast
/// raised just before a navigation is still on screen after it.
///
/// Inside the router rather than beside it, because signing out navigates —
/// `use_navigator` panics outside a `Router` descendant.
#[component]
pub fn AppOverlays() -> Element {
    let mut app_state = use_context::<AppState>();
    let (message, color) = app_state.toast.read().clone();
    let navigator = use_navigator();

    rsx! {
        Toast {
            open: app_state.toast_open,
            message,
            color,
            position: ToastPosition::Bottom,
        }

        ConfirmModal {
            open: app_state.sign_out_confirm_open,
            title: "Sign out?".to_string(),
            description: rsx! { "This guest account and its notes will not be recoverable." },
            confirm_text: "Sign out".to_string(),
            on_confirm: move |_| {
                spawn(async move {
                    let _ = sign_out().await;
                    app_state.user.set(None);
                    // Clearing `user` before navigating, so no screen renders
                    // for a frame against an account the server has already
                    // forgotten. The splash then re-asks and routes onward.
                    navigator.push(Route::Splash {});
                });
            },
        }
    }
}
