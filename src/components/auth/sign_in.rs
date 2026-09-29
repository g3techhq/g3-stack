use crate::{app::Route, auth::sign_in_guest, components::shared::error_message, state::AppState};
use dioxus::prelude::*;
use g3_route_transitions::animated_navigate;
use g3_ui::{
    Button, ButtonExpand, ButtonSize, Card, Content, ContentWidth, Header, Stack, Text, TextTone,
    use_toast,
};

/// The only sign-in the template ships.
///
/// Guest sign-in creates a real account and a real session, which is what
/// lets every other screen assume `session_user` exists rather than carrying
/// a signed-out branch. Adding email or an OAuth provider means another
/// server function beside `sign_in_guest` and another button here — the
/// session machinery underneath does not change. See docs/authentication.md.
#[component]
pub fn SignIn() -> Element {
    let mut app_state = use_context::<AppState>();
    let toast = use_toast();
    let mut signing_in = use_signal(|| false);

    let continue_as_guest = move |_| async move {
        signing_in.set(true);
        match sign_in_guest().await {
            Ok(user) => {
                app_state.apply_user(user);
                // `Notes` declares `handoff_from = SignIn`, so Back from the
                // app leaves it rather than returning to a sign-in screen for
                // an account that is signed in.
                animated_navigate(Route::Notes { filter: None }).await;
            }
            Err(error) => {
                signing_in.set(false);
                toast.error(format!("Could not sign in: {}", error_message(&error)));
            }
        }
    };

    rsx! {
        Header { title: "g3 stack" }
        // `Readable` keeps the card at a comfortable width on a desktop,
        // where stretched across the page it would read as a form field
        // rather than a welcome.
        Content { width: ContentWidth::Readable,
            Stack {
                Card { title: "Welcome",
                    Text { tone: TextTone::Secondary,
                        "This is the g3 stack starter. Continue as a guest to look around. "
                        "An account is created for you, so everything past this screen "
                        "behaves the way it will for a real user."
                    }
                }

                Button {
                    size: ButtonSize::Lg,
                    expand: ButtonExpand::Block,
                    loading: signing_in(),
                    onclick: continue_as_guest,
                    "Continue as guest"
                }
            }
        }
    }
}
