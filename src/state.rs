//! App-wide state.
//!
//! The rule this template follows: **per-screen state belongs in the URL,
//! cross-cutting state belongs here.** A filter, a selected tab, a search
//! term — those go in the route (see `Route` in `app.rs`), so they survive a
//! refresh, work with browser back, and can be linked to. `AppState` is only
//! for things genuinely shared across screens: who is signed in, what the
//! theme is, and the overlays any screen can raise.

use crate::db::{AppearanceMode, ColorScheme, User, get_current_user, update_appearance};
use dioxus::prelude::*;
use g3_ui::{ComponentMode, StatusColor};

/// Every field is a `Signal`, which is `Copy`, so `AppState` itself is `Copy`
/// — it can be captured into `move` closures and nested `spawn(async move
/// { .. })` blocks without any borrow-checker argument about who owns it.
/// That is the whole reason this is a struct of signals rather than a signal
/// of a struct.
#[derive(Clone, Copy)]
pub struct AppState {
    pub user: Signal<Option<User>>,
    pub mode: Signal<ComponentMode>,
    pub color_scheme: Signal<ColorScheme>,

    /// The toast every screen shares. One instance is mounted in
    /// `AppOverlays`; screens call `show_toast` rather than each rendering
    /// their own, so a toast raised just before a navigation survives it.
    pub toast_open: Signal<bool>,
    pub toast: Signal<(String, StatusColor)>,

    pub sign_out_confirm_open: Signal<bool>,

    /// Bumped after any mutation. Screens that display data derived from
    /// something another screen can change include a read of this in their
    /// `use_resource` closure — purely to pick up the dependency — so they
    /// refetch when it moves.
    ///
    /// A deliberately blunt instrument. It refetches more than strictly
    /// necessary, and that is the trade: the alternative is a per-entity
    /// cache-invalidation scheme, which is a great deal of machinery to get
    /// subtly wrong in an app this size. Reach for something finer when a
    /// screen is measurably too slow, not before.
    pub data_version: Signal<u64>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            user: Signal::new(None),
            mode: Signal::new(ComponentMode::Ios),
            color_scheme: Signal::new(ColorScheme::Light),
            toast_open: Signal::new(false),
            toast: Signal::new((String::new(), StatusColor::Neutral)),
            sign_out_confirm_open: Signal::new(false),
            data_version: Signal::new(0),
        }
    }
}

impl AppState {
    pub fn show_toast(&mut self, message: impl Into<String>, color: StatusColor) {
        self.toast.set((message.into(), color));
        self.toast_open.set(true);
    }

    pub fn bump_data(&mut self) {
        self.data_version += 1;
    }

    /// Makes `user` the signed-in account and adopts its saved appearance.
    ///
    /// Called with whatever the server said: at startup by
    /// `AppStateProvider`, and by `SignIn` with the account it just created.
    pub fn apply_user(&mut self, user: User) {
        let mode = to_component_mode(user.appearance_mode);
        apply_mode(mode);
        self.mode.set(mode);
        self.color_scheme.set(user.color_scheme);
        self.user.set(Some(user));
    }

    /// Applies an appearance change immediately and saves it in the
    /// background.
    ///
    /// Optimistic on purpose: a theme toggle that waits for a round trip
    /// feels broken. If the save fails the next page load reverts it, which
    /// is the right amount of ceremony for a preference.
    pub fn set_appearance(&mut self, mode: ComponentMode, scheme: ColorScheme) {
        apply_mode(mode);
        self.mode.set(mode);
        self.color_scheme.set(scheme);

        let appearance = to_appearance_mode(mode);
        spawn(async move {
            let _ = update_appearance(appearance, scheme).await;
        });
    }
}

pub fn to_component_mode(mode: AppearanceMode) -> ComponentMode {
    match mode {
        AppearanceMode::Ios => ComponentMode::Ios,
        AppearanceMode::Md => ComponentMode::Md,
    }
}

pub fn to_appearance_mode(mode: ComponentMode) -> AppearanceMode {
    match mode {
        ComponentMode::Ios => AppearanceMode::Ios,
        ComponentMode::Md => AppearanceMode::Md,
    }
}

/// Sets the platform in both libraries at once.
///
/// g3-ui's `mode` and g3-route-transitions' `Platform` are separate
/// thread-local globals that a component reads while rendering, and they
/// answer different questions ("which aesthetic do I draw?" and "which motion
/// language do I animate with?"). They have to be set together, or a screen
/// drawn in Material style animates with iOS motion.
pub fn apply_mode(mode: ComponentMode) {
    g3_ui::set_mode(mode);
    g3_route_transitions::set_platform(match mode {
        ComponentMode::Ios => g3_route_transitions::Platform::Ios,
        ComponentMode::Md => g3_route_transitions::Platform::Md,
    });
}

/// Provides `AppState` and loads the signed-in user into it.
///
/// Mounted above the `Router` so the state outlives navigation.
#[component]
pub fn AppStateProvider(children: Element) -> Element {
    let mut app_state = use_context_provider(AppState::default);

    // `peek`, not a tracked read. This component writes `mode` below, and
    // subscribing to it here would make that write re-render this very
    // component: a loop that burns frames on the client and, during SSR —
    // which renders until nothing is dirty — never returns a response at all.
    apply_mode(*app_state.mode.peek());

    // A server future rather than `use_resource`: the server renders with the
    // answer already in hand and ships it with the HTML, so a returning user's
    // first paint is already in their saved theme instead of the default
    // swapping to theirs once the WASM boots. `/api/v1/user` is outside the
    // auth guard, so a signed-out visitor gets `Ok(None)` rather than a
    // redirect this cannot decode.
    let current_user = use_server_future(get_current_user)?;

    // Runs once per answer, not once per render: nothing else this component
    // reads changes. That matters, because re-applying the stored appearance
    // after the user had already toggled it would read as a switch that flips
    // itself back.
    if let Some(Ok(Some(user))) = current_user.read().clone()
        && app_state.user.peek().as_ref() != Some(&user)
    {
        app_state.apply_user(user);
    }

    rsx! {
        {children}
    }
}
