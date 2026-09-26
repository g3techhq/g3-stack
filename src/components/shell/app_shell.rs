use crate::{app::Route, components::NotesToolbar};
use dioxus::prelude::*;
use dioxus_icons::lucide::{NotebookPen, Plus, Settings as SettingsIcon};
use g3_route_transitions::{RouteTransitionPage, animated_navigate};
use g3_ui::{
    Body, Button, ButtonStyle, Header, Navbar, NavbarTab, NavbarTabBar, NavbarTabDesktopPlacement,
};

/// The persistent tab shell: header, scrollable body, and tab bar.
///
/// Attached to the routes it wraps with `#[layout(AppShell)]` in `app.rs`, so
/// switching tabs re-renders the `Outlet` without remounting the chrome
/// around it. One `match` over the route derives the header for every tab —
/// worth doing here rather than giving each screen its own header, because
/// the header is part of the shell that persists across the transition.
///
/// `Navbar` is what widens: the same tree renders as a bottom tab bar on a
/// phone and as a left rail from 48rem. That is a container query on the
/// shell's own width, not the viewport's, so this stays in its phone layout
/// when embedded in something wide.
#[component]
pub fn AppShell() -> Element {
    let route: Route = use_route();

    let title = match &route {
        Route::Notes { .. } => "Notes",
        Route::Settings {} => "Settings",
        _ => "",
    };

    let toolbar = match &route {
        Route::Notes { filter } => {
            let filter = filter.unwrap_or_default();
            Some(rsx! {
                NotesToolbar {
                    filter,
                    on_change: move |filter| {
                        // `#[transition(root, replace)]` on this route is what
                        // makes changing the filter skip the animation and
                        // replace the history entry instead of pushing one.
                        spawn(animated_navigate(Route::Notes { filter: Some(filter) }));
                    },
                }
            })
        }
        _ => None,
    };

    let end_button = matches!(route, Route::Notes { .. }).then(|| {
        rsx! {
            Button {
                style: ButtonStyle::Clear,
                aria_label: Some("New note".to_string()),
                onclick: move |_| { spawn(animated_navigate(Route::NewNote {})); },
                Plus { size: 20 }
            }
        }
    });

    rsx! {
        RouteTransitionPage {
            Navbar {
                Header { title: title.to_string(), toolbar, end_button }
                Body {
                    Outlet::<Route> {}
                }
                NavbarTabBar {
                    NavbarTab {
                        label: "Notes".to_string(),
                        selected: matches!(route, Route::Notes { .. }),
                        icon: rsx! { NotebookPen { size: 24 } },
                        onclick: move |_| {
                            spawn(animated_navigate(Route::Notes { filter: None }));
                        },
                    }
                    NavbarTab {
                        label: "Settings".to_string(),
                        selected: matches!(route, Route::Settings {}),
                        // Secondary destinations stay at the end of the mobile
                        // tab bar but move to the bottom of the desktop rail,
                        // the way a settings entry does in a native sidebar.
                        desktop_placement: NavbarTabDesktopPlacement::Bottom,
                        icon: rsx! { SettingsIcon { size: 24 } },
                        onclick: move |_| { spawn(animated_navigate(Route::Settings {})); },
                    }
                }
            }
        }
    }
}

/// The header for screens outside the tab shell — a pushed page or a sheet.
///
/// Those screens have no tab bar (that is the point of pushing over it), so
/// they each render their own `Navbar`/`Header`/`Body` with a back button in
/// the start slot. This wrapper keeps that from being copy-pasted five times.
#[component]
pub fn PageShell(
    title: String,
    #[props(default)] end_button: Option<Element>,
    children: Element,
) -> Element {
    rsx! {
        Navbar {
            Header {
                title,
                start_button: rsx! { crate::components::BackButton {} },
                end_button,
            }
            Body {
                {children}
            }
        }
    }
}
