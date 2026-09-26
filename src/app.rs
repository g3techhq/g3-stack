//! Routes, transitions, and the theme.
//!
//! This is the file to read first. The `Route` enum below is the app's map:
//! every screen, the URL that reaches it, and — through `#[transition(..)]` —
//! how it animates in. No component contains animation code; it is all
//! declared here, next to the route it belongs to.

use crate::{
    components::{
        AppOverlays, AppShell, EditNote, NewNote, NoteDetail, Notes, Settings, SignIn, Splash,
    },
    db::ColorScheme,
    state::{AppState, AppStateProvider},
};
use dioxus::prelude::*;
use g3_auth::PublicRoutes;
use g3_native_plugins::NativePluginsProvider;
#[cfg(test)]
use g3_route_transitions::{NavigationAnimation, RouteTransitions};
use g3_route_transitions::{RouteTransitionPage, route_transitions, use_native_back_navigation};
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

impl NotesFilter {
    pub fn segment_index(&self) -> usize {
        match self {
            NotesFilter::All => 0,
            NotesFilter::Pinned => 1,
        }
    }

    pub fn from_segment_index(index: usize) -> Self {
        match index {
            1 => NotesFilter::Pinned,
            _ => NotesFilter::All,
        }
    }
}

/// The route table.
///
/// `#[route_transitions]` reads the `#[transition(..)]` attributes below and
/// generates the metadata `animated_navigate` uses to pick an animation. The
/// vocabulary:
///
/// - `root` — a stable destination, i.e. a bottom tab. Root-to-root fades.
/// - `pushed` — a full-screen page above a root. In pushes left, back pushes
///   right (with iOS parallax or Material shared-axis, per platform).
/// - `cover` — a sheet or modal. Rises from the bottom, dismisses downward.
/// - `morph` — a page that grows out of a card on the route before it.
/// - `base` — an ordinary page. The default; you rarely write it.
///
/// And the modifiers:
///
/// - `replace` makes a change *within* the same variant skip the animation
///   and replace history instead of pushing it. That is what stops a
///   segmented filter from cross-fading the whole page on every tap, and
///   stops back from walking through every filter the user tried.
///   `replace(key = id)` narrows it to changes where the identity fields
///   match.
/// - `replaces = Route` (or a tuple) hands the listed route off to this one:
///   arriving from it replaces its history entry. Used below so Back never
///   lands on the splash or the sign-in screen, both of which would
///   immediately send you forward again.
/// - `forward = Route` (or a tuple) declares a drill-down, so
///   pushed-to-pushed navigation knows which direction it is going. It
///   overrides the destination's own kind, so never point it at a `cover`
///   route — the sheet would slide in from the side instead of rising.
/// - `push(group = name, order = field)` orders peer routes, such as the
///   steps of a wizard, so moving between them slides the right way.
///
/// Anything with no more specific match falls back to `Fade`.
///
/// `#[public]` marks the pages a signed-out visitor may load (see
/// `auth::session`); a signed-out load of any other page goes to the splash.
#[route_transitions]
#[derive(Debug, Clone, Routable, PartialEq, PublicRoutes)]
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

        #[transition(replaces = Splash)]
        #[public]
        #[route("/signin")]
        SignIn {},

        #[layout(AppShell)]
            #[transition(root, replace, replaces = (Splash, SignIn))]
            #[route("/notes?:filter")]
            Notes { filter: Option<NotesFilter> },

            #[transition(root)]
            #[route("/settings")]
            Settings {},
        #[end_layout]

        // Sheets. Reachable from both the list and the detail page, so they
        // sit outside the tab shell rather than inside either one.
        #[transition(cover)]
        #[route("/notes/new")]
        NewNote {},
        #[transition(cover)]
        #[route("/notes/:id/edit")]
        EditNote { id: String },

        // No `#[end_layout]` after this one, or after `RootLayout`: a layout
        // opened and never closed simply runs to the end of the enum, and the
        // macro rejects a trailing close with nothing after it.
        #[layout(PushedPageLayout)]
            // Deliberately no `forward = EditNote`. `forward` declares a
            // drill-down *push*, and it wins over the destination's own kind —
            // naming a `cover` route there makes the sheet slide in from the
            // side instead of rising. Reserve it for pushed-to-pushed.
            #[transition(pushed)]
            #[route("/notes/:id")]
            NoteDetail { id: String },
}

/// Wraps every route so `use_native_back_navigation` is installed exactly
/// once, inside the router's context. That is what connects Android's system
/// Back button and iOS's left-edge swipe to the same animated pop the app's
/// own back button performs.
///
/// Also where the app-wide overlays are mounted. They have to be *inside* the
/// router — `AppOverlays` navigates on sign-out, and `use_navigator` panics
/// outside a `Router` descendant — while still sitting above every route, so
/// their backdrops are not clipped by a screen's scroll container and a toast
/// raised just before a navigation survives it.
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
        AppOverlays {}
    }
}

/// Gives pushed routes one stable, viewport-sized snapshot.
///
/// Without it, a page whose header, body, and tab bar are separately marked
/// produces three independent snapshots that can slide over each other, or
/// expose the WebView's background between them.
#[component]
fn PushedPageLayout() -> Element {
    rsx! {
        RouteTransitionPage {
            Outlet::<Route> {}
        }
    }
}

/// Your brand, in one function.
///
/// `Theme`'s fields are all public, so a whole custom palette is struct-update
/// syntax over a preset. `with_focused` alone changes the accent that tints
/// focus rings, selected segments, active toggles, and links — which is most
/// of what makes an app look like itself. docs/styling.md walks through a full
/// palette.
pub fn app_theme(scheme: ColorScheme) -> Theme {
    match scheme {
        ColorScheme::Light => Theme::default_light().with_focused("#2563eb"),
        ColorScheme::Dark => Theme::default_dark().with_focused("#3b82f6"),
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

        // Works around g3-ui 0.3.0. `AppWrapper` links its stylesheet at
        // runtime on every non-wasm target, which includes the *server* build,
        // but not in the web client. The server therefore writes one more
        // head element to the hydration stream than the client reads, and
        // every component after it decodes its state from the wrong slot
        // (logged as "Error deserializing data ... CapturedError"). Rendering
        // the same link here on the web client, in the same position, evens
        // the count. The browser already has the file, so it costs nothing.
        // Delete this once g3-ui gates that link to match.
        if cfg!(target_arch = "wasm32") {
            document::Link { rel: "stylesheet", href: g3_ui::UI_CSS }
        }

        AppWrapper { theme: app_theme(color_scheme), mode, disable_text_selection: true,
            // Mounted unconditionally. The provider itself renders nothing but
            // its children, and only installs the plugin context on targets
            // that have one, so the tree is the same shape everywhere — which
            // is what hydration needs.
            NativePluginsProvider {
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
        // Peer roots have no spatial relationship, so sliding between them
        // would imply a hierarchy that does not exist.
        assert_eq!(
            notes().transition_to(&Route::Settings {}),
            NavigationAnimation::Fade
        );
        assert!(!notes().replaces_history(&Route::Settings {}));
    }

    #[test]
    fn changing_the_filter_neither_animates_nor_grows_history() {
        let pinned = Route::Notes {
            filter: Some(NotesFilter::Pinned),
        };

        assert_eq!(notes().transition_to(&pinned), NavigationAnimation::None);
        assert!(notes().replaces_history(&pinned));
    }

    #[test]
    fn opening_a_note_pushes_and_back_reverses_it() {
        assert_eq!(
            notes().transition_to(&detail()),
            NavigationAnimation::PushLeft
        );
        assert_eq!(
            detail().transition_to(&notes()),
            NavigationAnimation::PushRight
        );
    }

    #[test]
    fn the_editor_rises_as_a_sheet_from_wherever_it_was_opened() {
        let editor = Route::EditNote {
            id: "abc".to_string(),
        };

        assert_eq!(
            detail().transition_to(&editor),
            NavigationAnimation::CoverUp
        );
        assert_eq!(
            editor.transition_to(&detail()),
            NavigationAnimation::UncoverDown
        );
        // Back out of a sheet dismisses downward even when there is no
        // recorded history to pop to.
        assert_eq!(editor.transition_back(), NavigationAnimation::UncoverDown);
    }

    #[test]
    fn the_new_note_sheet_covers_the_list() {
        assert_eq!(
            notes().transition_to(&Route::NewNote {}),
            NavigationAnimation::CoverUp
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
