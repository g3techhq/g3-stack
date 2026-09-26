use crate::{app::Route, auth::is_signed_in};
use dioxus::prelude::*;
use g3_route_transitions::animated_navigate;
use g3_ui::{Body, Button, Card, Navbar, Spinner};

/// The app's front door, at `/`, and the only place that decides between the
/// app and the sign-in screen.
///
/// It exists because the *server* cannot tell a stale cookie from a
/// never-signed-in visitor without asking the database, and answering that
/// inside a middleware would mean a database round trip on every request. So
/// `auth_check` redirects signed-out page loads here, a native build starts
/// here, and this screen asks once.
///
/// Keeping it in one place matters more than it looks: a second guard
/// elsewhere in the app would be a second source of truth about the same
/// question, and any disagreement between them throws a signed-in visitor
/// back out of the app.
#[component]
pub fn Splash() -> Element {
    let mut signed_in = use_resource(|| async { is_signed_in().await });

    use_effect(move || {
        let destination = match *signed_in.read() {
            Some(Ok(true)) => Route::Notes { filter: None },
            Some(Ok(false)) => Route::SignIn {},
            // Handled in the markup below. Guessing here would either strand a
            // signed-in visitor on the sign-in screen or send a signed-out one
            // into a wall of 401s.
            Some(Err(_)) | None => return,
        };
        // Both destinations declare `replaces = Splash`, so this entry leaves
        // history and Back does not return to a screen that would only
        // forward them again.
        spawn(animated_navigate(destination));
    });

    let unreachable = matches!(*signed_in.read(), Some(Err(_)));

    rsx! {
        Navbar {
            Body {
                if unreachable {
                    // Most often the backend restarting under `dx serve`, or a
                    // phone whose SERVER_URL points somewhere it cannot reach.
                    Card { title: "Can't reach the server",
                        p { class: "g3-message-text g3-message-text-muted",
                            "Check that it is running and try again."
                        }
                        Button { expand: true, onclick: move |_| signed_in.restart(), "Try again" }
                    }
                } else {
                    Spinner { center: true }
                }
            }
        }
    }
}
