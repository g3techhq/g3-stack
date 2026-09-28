//! Routes, transitions, and the theme.
//!
//! This is the file to read first. The `Route` enum below is the app's map:
//! every screen, the URL that reaches it, and — through `#[transition(..)]` —
//! how it animates in. No component contains animation code; it is all
//! declared here, next to the route it belongs to.

use crate::{
    components::{
        AppShell, EditNote, NewNote, NoteDetail, Notes, Settings, SheetShell, SignIn, Splash,
    },
    db::ColorScheme,
    state::{AppState, AppStateProvider},
};
use dioxus::prelude::*;
use g3_auth::PublicRoutes;
use g3_native_plugins::NativePluginsProvider;
use g3_route_transitions::{
    RouteTransitions, use_browser_history_transitions, use_native_back_navigation,
};
use g3_ui::{AppWrapper, Theme};
use strum_macros::{Display, EnumString};

/// Blocks first paint on the stylesheet for web builds. The runtime
/// `document::Link` below is not redundant with it: desktop and mobile
/// bundles only collect assets something links at runtime, so the static head
/// entry alone never reaches them.
const TAILWIND_CSS: Asset = asset!(
    "/assets/tailwind.css",
    AssetOptions::css().with_static_head(true)
);

/// Declared rather than left to the browser, which otherwise requests
/// `/favicon.ico` on every load and takes a 404 for it.
const FAVICON: Asset = asset!("/assets/logo.svg");

/// Which notes the list shows.
///
/// In the URL rather than in `AppState`, which is the pattern to copy for any
/// per-screen filter or tab: it survives a refresh, browser back moves
/// through it, and a link to a filtered view is just a link.
#[derive(Default, Display, Clone, Copy, PartialEq, Eq, Debug, EnumString)]
pub enum NotesFilter {
    #[default]
    All,
    Pinned,
}

/// The route table.
///
/// `#[derive(RouteTransitions)]` reads the `#[transition(..)]` attributes
/// below, and `animated_navigate` picks each animation from them. The layers:
///
/// - `layer = stack_root`: a tab. Tab to tab cross-fades.
/// - `layer = stack_page`: a page pushed above a tab. It slides in, and Back
///   slides it out, in the platform's own motion.
/// - `layer = sheet`: rises from the bottom and dismisses downward.
/// - no layer: an ordinary page, which cross-fades.
///
/// And the options:
///
/// - `history = replace` makes a move *within* the same variant instant, and
///   replaces the history entry instead of pushing one. That is what stops a
///   filter from cross-fading the page on every tap, and Back from walking
///   through every filter the user tried.
/// - `handoff_from = Route` (or a tuple): arriving from the listed route
///   replaces its history entry, so Back never lands on the splash or the
///   sign-in screen, both of which would send you straight forward again.
/// - `forward_to = Route` (or a tuple) declares a drill-down between two
///   pushed pages, so the push runs the right way. Not needed from a tab or
///   to a sheet: the layers already say which way those go.
///
/// The full rule table is in docs/navigation.md.
///
/// `#[public]` marks the pages a signed-out visitor may load (see
/// `auth::session`); a signed-out load of any other page goes to the splash.
#[derive(Debug, Clone, Routable, PartialEq, RouteTransitions, PublicRoutes)]
#[rustfmt::skip]
pub enum Route {
    // The splash owns `/`, and every unknown path lands there too. It is where
    // the signed-in question gets asked, and it has to be `/` rather than
    // somewhere the auth guard redirects to: a native build never makes a
    // document request for the server to redirect. It simply starts its
    // router at `/`.
    #[redirect("/:.._segments", |_segments: Vec<String>| Route::Splash {})]
    #[layout(RootLayout)]
        #[public]
        #[route("/")]
        Splash {},

        #[transition(handoff_from = Splash)]
        #[public]
        #[route("/signin")]
        SignIn {},

        #[layout(AppShell)]
            #[transition(layer = stack_root, history = replace, handoff_from = (Splash, SignIn))]
            #[route("/notes?:filter")]
            Notes { filter: Option<NotesFilter> },

            #[transition(layer = stack_root)]
            #[route("/settings")]
            Settings {},
        #[end_layout]

        // Sheets rise beside the desktop rail rather than over it, so they get
        // a layout that renders the same rail (hidden on phones). Reachable
        // from both the list and the detail page, so they belong to neither.
        #[layout(SheetShell)]
            #[transition(layer = sheet)]
            #[route("/notes/new")]
            NewNote {},

            #[transition(layer = sheet)]
            #[route("/notes/:id/edit")]
            EditNote { id: String },
        #[end_layout]

        // Pushed pages reuse the tab shell, so the navigation stays put while
        // only the page slides. The page supplies its own header.
        //
        // No `#[end_layout]` after this one, or after `RootLayout`: a layout
        // opened and never closed runs to the end of the enum, and the macro
        // rejects a trailing close with nothing after it.
        #[layout(AppShell)]
            #[transition(layer = stack_page)]
            #[route("/notes/:id")]
            NoteDetail { id: String },
}

/// Wraps every route so `use_native_back_navigation` is installed exactly
/// once, inside the router's context. That is what connects Android's system
/// Back button and iOS's left-edge swipe to the same animated pop the app's
/// own back button performs.
///
/// Deliberately does no auth work. The signed-out decision lives in
/// `Route::Splash` — one place that asks the server once. A guard here would
/// be a second source of truth about the same question, and any disagreement
/// between the two throws a signed-in visitor out of the app.
#[component]
fn RootLayout() -> Element {
    use_native_back_navigation::<Route>();

    rsx! {
        Outlet::<Route> {}
    }
}

/// Your brand, in one function.
///
/// `Theme`'s fields are all public, so a whole custom palette is struct-update
/// syntax over a preset. `with_accent` alone changes the color that tints
/// focus rings, selected segments, active toggles, and links — which is most
/// of what makes an app look like itself. docs/styling.md walks through a full
/// palette.
pub fn app_theme(scheme: ColorScheme) -> Theme {
    match scheme {
        ColorScheme::Light => Theme::default_light().with_accent("#2563eb"),
        ColorScheme::Dark => Theme::default_dark().with_accent("#3b82f6"),
    }
}

#[component]
pub fn App() -> Element {
    // First thing: names the device's cache store (rename it for your app),
    // and refetches what mounted screens show when the app comes back into
    // view. See `use_cached` in the notes screens.
    g3_cache::use_client_cache(g3_cache::CacheConfig::new("g3-app"));

    rsx! {
        AppStateProvider {
            ThemedShell {}
        }
    }
}

/// Reads the appearance signals and passes their current values into g3-ui.
///
/// Separate from `App` because it has to be *inside* `AppStateProvider` to
/// read the context that provider supplies. `AppWrapper` writes the theme as
/// CSS custom properties on the shell element, so a new `Theme` re-themes the
/// whole tree without remounting it — navigation, scroll position, and
/// half-typed form state all survive a light/dark switch.
#[component]
fn ThemedShell() -> Element {
    let app_state = use_context::<AppState>();
    let mode = (app_state.mode)();
    let color_scheme = (app_state.color_scheme)();
    // The browser's own Back and Forward buttons animate like the app's.
    use_browser_history_transitions::<Route>();

    rsx! {
        // Here, below `AppStateProvider`, rather than beside it in `App`.
        // Fullstack hydrates head elements and server futures from one ordered
        // stream. The provider's `use_server_future` suspends on the server,
        // so anything *beside* it is written to that stream before its
        // children, while the client — which already has the data — renders
        // the children first. The two then disagree about which entry is
        // which, and the client fails to decode one component's state as
        // another's. Everything inside the provider is ordered the same way
        // on both sides.
        document::Link { rel: "icon", r#type: "image/svg+xml", href: FAVICON }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        // Works around g3-ui 0.4.2. `AppWrapper` links its stylesheet at
        // runtime on every non-wasm target, which includes the *server* build,
        // but not in the web client. The server therefore writes one more
        // head element to the hydration stream than the client reads, and
        // every component after it decodes its state from the wrong slot
        // (logged as "Error deserializing data ... CapturedError"). Rendering
        // the same link here on the web client, in the same position, evens
        // the count. The browser already has the file, so it costs nothing.
        // Delete this once a g3-ui release gates that link to match.
        if cfg!(target_arch = "wasm32") {
            document::Link { rel: "stylesheet", href: g3_ui::UI_CSS }
        }

        // Mounted unconditionally. The provider renders nothing but its
        // children, and only installs the plugin context on targets that have
        // one, so the tree is the same shape everywhere — which is what
        // hydration needs.
        NativePluginsProvider {
            // `AppWrapper` also hosts the toasts and alerts that `use_toast`
            // and `use_alert` open, above every screen.
            AppWrapper { theme: app_theme(color_scheme), mode, text_selection: false,
                Router::<Route> {}
            }
        }
    }
}

/// Transitions are declarative, so they are also testable — no browser
/// needed. These lock in the intent behind each `#[transition(..)]` above, so
/// that adding a route later cannot silently change how an existing one
/// animates.
#[cfg(test)]
mod transition_tests {
    use super::*;
    use g3_route_transitions::NavigationTransition;

    fn notes() -> Route {
        Route::Notes { filter: None }
    }

    fn detail() -> Route {
        Route::NoteDetail {
            id: "abc".to_string(),
        }
    }

    #[test]
    fn tabs_cross_fade_rather_than_sliding() {
        // Peer tabs have no spatial relationship, so sliding between them
        // would imply a hierarchy that does not exist.
        assert_eq!(
            notes().transition_to(&Route::Settings {}),
            NavigationTransition::CrossFade
        );
        assert!(!notes().replaces_history(&Route::Settings {}));
    }

    #[test]
    fn changing_the_filter_neither_animates_nor_grows_history() {
        let pinned = Route::Notes {
            filter: Some(NotesFilter::Pinned),
        };

        assert_eq!(notes().transition_to(&pinned), NavigationTransition::None);
        assert!(notes().replaces_history(&pinned));
    }

    #[test]
    fn opening_a_note_pushes_and_back_reverses_it() {
        assert_eq!(
            notes().transition_to(&detail()),
            NavigationTransition::Forward
        );
        assert_eq!(
            detail().transition_to(&notes()),
            NavigationTransition::Backward
        );
    }

    #[test]
    fn the_editor_rises_as_a_sheet_from_wherever_it_was_opened() {
        let editor = Route::EditNote {
            id: "abc".to_string(),
        };

        assert_eq!(
            detail().transition_to(&editor),
            NavigationTransition::PresentSheet
        );
        assert_eq!(
            editor.transition_to(&detail()),
            NavigationTransition::DismissSheet
        );
        // Back out of a sheet dismisses downward even when there is no
        // recorded history to pop to.
        assert_eq!(editor.transition_back(), NavigationTransition::DismissSheet);
    }

    #[test]
    fn the_new_note_sheet_covers_the_list() {
        assert_eq!(
            notes().transition_to(&Route::NewNote {}),
            NavigationTransition::PresentSheet
        );
    }

    #[test]
    fn back_never_returns_to_the_splash_or_sign_in() {
        // Both would immediately navigate forward again, so the screen after
        // them takes their place in history instead of stacking on top.
        assert!(Route::Splash {}.replaces_history(&Route::SignIn {}));
        assert!(Route::Splash {}.replaces_history(&notes()));
        assert!(Route::SignIn {}.replaces_history(&notes()));

        // Only from those two. An ordinary visit to the list from a sheet
        // or a detail page still adds to history.
        assert!(!detail().replaces_history(&notes()));
    }

    #[test]
    fn the_splash_is_the_root_of_the_url_space() {
        // The auth guard sends signed-out page loads here, and a native
        // build starts its router here. Both depend on it staying `/`.
        assert_eq!(Route::Splash {}.to_string(), "/");
    }
}
