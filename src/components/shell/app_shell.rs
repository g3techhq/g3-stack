use crate::{app::Route, components::NotesToolbar};
use dioxus::prelude::*;
use dioxus_icons::lucide::{NotebookPen, Plus, Settings as SettingsIcon};
use g3_route_transitions::{RouteTransitionBaseRegion, RouteTransitionPage, animated_navigate};
use g3_ui::{
    AdaptiveNav, AdaptiveNavCompact, Button, ButtonFill, Content, ContentWidth, Header, NavItem,
    NavItemGroup, TabLayout,
};

/// The persistent tab shell (`#[layout(AppShell)]`): a header with the
/// active tab's title and filter, the tab's page, and the navigation — bottom
/// tabs on a phone, a rail on a wide screen.
///
/// Only the page transitions. The navigation sits outside
/// [`RouteTransitionPage`], so it stays still while a page moves under it,
/// the way a native tab bar does.
///
/// Pushed pages (`layer = stack_page`) render inside this shell too, with
/// their own header, so the rail stays put while they slide. One `match` over
/// the route derives the header for every tab: the header is part of the
/// shell, which persists across the transition.
///
/// `AdaptiveNav` is what widens: the same tree renders as a bottom tab bar on
/// a phone and as a left rail from 48rem. That is a container query on the
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
        Route::Notes { filter } => Some(rsx! {
            NotesToolbar {
                filter: filter.unwrap_or_default(),
                on_change: move |filter| {
                    // `history = replace` on this route is what makes changing
                    // the filter skip the animation and replace the history
                    // entry instead of pushing one.
                    spawn(animated_navigate(Route::Notes { filter: Some(filter) }));
                },
            }
        }),
        _ => None,
    };

    let end = matches!(route, Route::Notes { .. }).then(|| {
        rsx! {
            Button {
                fill: ButtonFill::Clear,
                aria_label: "New note",
                onclick: move |_| animated_navigate(Route::NewNote {}),
                Plus { size: 22 }
            }
        }
    });

    let is_tab = matches!(route, Route::Notes { .. } | Route::Settings {});

    rsx! {
        TabLayout { route_transition_base: false,
            RouteTransitionPage {
                if is_tab {
                    Header { title, toolbar, end }
                    RouteTransitionBaseRegion {
                        Content { width: ContentWidth::Readable,
                            Outlet::<Route> {}
                        }
                    }
                } else {
                    Outlet::<Route> {}
                }
            }
            ShellNav { route, compact: AdaptiveNavCompact::Bar }
        }
    }
}

/// Layout for routed sheets (`layer = sheet`): on a wide screen the sheet
/// takes the space beside the rail; on a phone it covers the whole screen,
/// bottom bar included.
///
/// The rail is persistent chrome, so both sides of a sheet transition have to
/// render it in the same place. A sheet outside any shell covers the rail,
/// which then fades out to the bare page background while the sheet rises.
#[component]
pub fn SheetShell() -> Element {
    let route: Route = use_route();

    rsx! {
        TabLayout { route_transition_base: false,
            Outlet::<Route> {}
            ShellNav { route, compact: AdaptiveNavCompact::Hidden }
        }
    }
}

/// One of the navigation's destinations.
#[derive(Clone, Copy, PartialEq, Eq)]
enum NavTab {
    Notes,
    Settings,
}

impl NavTab {
    /// The tab a route belongs to. A note, and its sheets, belong to Notes.
    fn of(route: &Route) -> Self {
        match route {
            Route::Settings {} => Self::Settings,
            _ => Self::Notes,
        }
    }
}

/// The app's navigation, shared by [`AppShell`] and [`SheetShell`] so the
/// rail is the same element on both sides of a sheet transition.
///
/// `route` is a `ReadSignal` because the memo below reads it: the memo then
/// reruns when the route changes, and the nav re-renders only when the lit
/// tab does.
#[component]
fn ShellNav(route: ReadSignal<Route>, compact: AdaptiveNavCompact) -> Element {
    let lit = use_memo(move || NavTab::of(&route.read()));

    rsx! {
        AdaptiveNav { compact,
            NavItem {
                label: "Notes",
                selected: lit() == NavTab::Notes,
                icon: rsx! { NotebookPen { size: 24 } },
                onclick: move |_| animated_navigate(Route::Notes { filter: None }),
            }
            NavItem {
                label: "Settings",
                // Secondary destinations stay at the end of the bottom bar but
                // move to the foot of the rail, where a settings entry sits in
                // a native sidebar.
                group: NavItemGroup::Secondary,
                selected: lit() == NavTab::Settings,
                icon: rsx! { SettingsIcon { size: 24 } },
                onclick: move |_| animated_navigate(Route::Settings {}),
            }
        }
    }
}
