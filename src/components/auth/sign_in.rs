use crate::{app::Route, auth::sign_in_guest, state::AppState};
use dioxus::prelude::*;
use g3_route_transitions::animated_navigate;
use g3_ui::{Body, Button, ButtonSize, Card, Navbar, StatusColor};

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
    let mut signing_in = use_signal(|| false);

    rsx! {
        Navbar {
            Body {
                // The one layout utility on this screen: a sign-in card
                // stretched across a desktop rail reads as a form field, not
                // a welcome.
                div { class: "mx-auto w-full max-w-md",
                    Card { title: "Welcome",
                        p { class: "g3-message-text g3-message-text-muted",
                            "This is the g3 stack starter. Continue as a guest to look around. "
                            "An account is created for you, so everything past this screen "
                            "behaves the way it will for a real user."
                        }
                    }

                    Button {
                        size: ButtonSize::Lg,
                        expand: true,
                        disabled: signing_in(),
                        onclick: move |_| {
                            signing_in.set(true);
                            spawn(async move {
                                match sign_in_guest().await {
                                    Ok(user) => {
                                        app_state.apply_user(user);
                                        // `Notes` declares `replaces = SignIn`, so Back from
                                        // the app leaves it rather than returning to a
                                        // sign-in screen for an account that is signed in.
                                        animated_navigate(Route::Notes { filter: None }).await;
                                    }
                                    Err(error) => {
                                        signing_in.set(false);
                                        app_state.show_toast(
                                            format!("Could not sign in: {error}"),
                                            StatusColor::Danger,
                                        );
                                    }
                                }
                            });
                        },
                        if signing_in() { "Signing in…" } else { "Continue as guest" }
                    }
                }
            }
        }
    }
}
