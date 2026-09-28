//! App-wide state.
//!
//! The rule this template follows: **per-screen state belongs in the URL,
//! cross-cutting state belongs here.** A filter, a selected tab, a search
//! term — those go in the route (see `Route` in `app.rs`), so they survive a
//! refresh, work with browser back, and can be linked to. `AppState` is only
//! for things genuinely shared across screens: who is signed in and what the
//! theme is. Toasts and alerts need nothing here: `use_toast` and
//! `use_alert` open them in the host `AppWrapper` provides.

use crate::data_change::DataChange;
use crate::db::{AppearanceMode, ColorScheme, User, get_current_user, update_appearance};
use dioxus::prelude::*;
use g3_ui::ComponentMode;

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

    /// Bumped after every mutation (see [`AppState::changed`]). A read that
    /// changes per keystroke, and so stays on `use_resource` rather than
    /// `use_cached`, reads this in its closure purely to pick up the
    /// dependency, so it refetches after a mutation made elsewhere.
    pub data_version: Signal<u64>,
}

impl AppState {
    /// Built inside `use_context_provider`'s initializer, so the signals
    /// belong to the provider's scope and live as long as it does.
    fn new() -> Self {
        Self {
            user: Signal::new(None),
            mode: Signal::new(ComponentMode::Ios),
            color_scheme: Signal::new(ColorScheme::Light),
            data_version: Signal::new(0),
        }
    }

    /// Call after a mutation succeeds, saying what it changed: marks the
    /// cached reads that can see it stale (see [`crate::data_change`]), so
    /// mounted screens refetch while still showing what they had.
    pub fn changed(&mut self, change: DataChange) {
        self.data_version += 1;
        crate::data_change::invalidate(change);
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
        ComponentMode::Md => g3_route_transitions::Platform::Material,
    });
}

/// Provides `AppState` and loads the signed-in user into it.
///
/// Mounted above the `Router` so the state outlives navigation.
#[component]
pub fn AppStateProvider(children: Element) -> Element {
    let mut app_state = use_context_provider(AppState::new);

    // `peek`, not a tracked read. This component writes `mode` below, and
    // subscribing to it here would make that write re-render this very
    // component: a loop that burns frames on the client and, during SSR —
    // which renders until nothing is dirty — never returns a response at all.
    apply_mode(*app_state.mode.peek());

    // Above the `?` below, like every hook here: a hook after an early return
    // runs on some renders and not others. The client cache holds one user's
    // data: empty it on sign-out, or when someone else signs in, before their
    // screens can show it. Nothing persists to disk until this has run.
    let user = app_state.user;
    use_effect(move || {
        let owner = user
            .read()
            .as_ref()
            .map(|user| crate::db::record_key(&user.id));
        spawn(g3_cache::set_cache_owner(owner));
    });

    // A server future rather than `use_resource`: the server renders with the
    // answer already in hand and ships it with the HTML, so a returning user's
    // first paint is already in their saved theme instead of the default
    // swapping to theirs once the WASM boots. `/api/v1/user` is
    // `#[g3_auth::public]`, so a signed-out visitor gets `Ok(None)` rather
    // than a `401`.
    let current_user = use_server_future(get_current_user)?;

    // The one signal written during render in this template, on purpose.
    // Effects never run during SSR, so an effect here would render the
    // server's HTML in the default theme and the saved one would flash in
    // after hydration. It is safe because it is guarded: it writes only when
    // the answer differs from what is applied, so it runs once per answer and
    // the re-render it causes writes nothing. Anywhere else, a signal write
    // belongs in an event handler or an effect (see docs/dioxus/patterns.md).
    if let Some(Ok(Some(user))) = &*current_user.read()
        && app_state.user.peek().as_ref() != Some(user)
    {
        app_state.apply_user(user.clone());
    }

    rsx! {
        {children}
    }
}
