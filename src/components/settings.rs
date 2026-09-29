use crate::{
    app::Route,
    auth::{delete_account, sign_out},
    components::shared::{confirm_destructive, error_message},
    db::{AppearanceMode, ColorScheme},
    state::AppState,
};
use dioxus::prelude::*;
use dioxus_icons::lucide::{ClipboardCopy, ExternalLink, Share2};
use g3_ui::{
    Button, ButtonExpand, ButtonFill, Card, Color, Item, ItemDetail, List, ListLines, ListVariant,
    SegmentButton, SegmentGroup, Stack, Text, TextTone, use_alert, use_toast,
};

/// Appearance, native-plugin demos, and the account.
///
/// The appearance controls are the interesting part: they drive both g3-ui's
/// component mode and g3-route-transitions' motion language from one place
/// (see `AppState::set_appearance`), so switching to Material restyles every
/// component *and* changes how the next navigation animates.
#[component]
pub fn Settings() -> Element {
    rsx! {
        Stack {
            Appearance {}
            NativePluginDemos {}
            Account {}
        }
    }
}

/// Split out so a theme change re-renders these controls, not the account
/// card and the plugin demos beside them.
///
/// Both groups show `AppState`'s own signals and only report a pick
/// (`defer_selection`): `set_appearance` applies and saves it, so there is no
/// second copy here to fall out of step with a preference loaded later.
#[component]
fn Appearance() -> Element {
    let mut app_state = use_context::<AppState>();

    rsx! {
        Card { title: "Appearance",
            Stack {
                Text { tone: TextTone::Secondary,
                    "iOS and Material are Ionic's two modes. Switching restyles every component "
                    "and changes how the next screen animates in. Auto follows the device."
                }
                SegmentGroup {
                    value: app_state.appearance,
                    label: "Style",
                    defer_selection: true,
                    onchange: move |appearance: AppearanceMode| {
                        let scheme = *app_state.color_scheme.peek();
                        app_state.set_appearance(appearance, scheme);
                    },
                    SegmentButton { value: AppearanceMode::Auto, "Auto" }
                    SegmentButton { value: AppearanceMode::Ios, "iOS" }
                    SegmentButton { value: AppearanceMode::Md, "Material" }
                }
                SegmentGroup {
                    value: app_state.color_scheme,
                    label: "Theme",
                    defer_selection: true,
                    onchange: move |scheme: ColorScheme| {
                        let appearance = *app_state.appearance.peek();
                        app_state.set_appearance(appearance, scheme);
                    },
                    SegmentButton { value: ColorScheme::Auto, "Auto" }
                    SegmentButton { value: ColorScheme::Dark, "Dark" }
                    SegmentButton { value: ColorScheme::Light, "Light" }
                }
            }
        }
    }
}

#[component]
fn Account() -> Element {
    let mut app_state = use_context::<AppState>();
    let navigator = use_navigator();
    let toast = use_toast();
    let alerts = use_alert();

    let sign_out_clicked = move |_| async move {
        if !confirm_destructive(
            alerts,
            "Sign out?",
            "This guest account and its notes will not be recoverable.",
            "Sign out",
        )
        .await
        {
            return;
        }
        let _ = sign_out().await;
        // Clearing `user` before navigating, so no screen renders for a
        // frame against an account the server has already forgotten. The
        // splash then re-asks and routes onward.
        app_state.user.set(None);
        navigator.push(Route::Splash {});
    };

    let delete_clicked = move |_| async move {
        if !confirm_destructive(
            alerts,
            "Delete your account?",
            "Your account and every note in it are removed for good. This cannot be undone.",
            "Delete account",
        )
        .await
        {
            return;
        }
        match delete_account().await {
            Ok(()) => {
                app_state.user.set(None);
                toast.show("Account deleted.");
                // `replace`: the page this entry points at belonged to an
                // account that no longer exists.
                navigator.replace(Route::Splash {});
            }
            Err(error) => {
                toast.error(format!(
                    "Could not delete the account: {}",
                    error_message(&error)
                ));
            }
        }
    };

    let user = app_state.user.read();
    let handle = user
        .as_ref()
        .map(|user| user.handle.as_str())
        .unwrap_or_default();

    rsx! {
        Card { title: "Account",
            List { variant: ListVariant::EdgeToEdge, lines: ListLines::Inset,
                Item { label: "Signed in as", metadata: handle }
                Item {
                    label: "Sign out",
                    detail: ItemDetail::Hide,
                    onclick: sign_out_clicked,
                }
            }
        }

        Button {
            fill: ButtonFill::Outline,
            color: Color::Danger,
            expand: ButtonExpand::Block,
            onclick: delete_clicked,
            "Delete account"
        }
    }
}

/// Exercises g3-native-plugins.
///
/// Note what is and is not gated. The `use_context` line and the plugin calls
/// are behind `cfg`, because `NativePlugins` only exists on targets that have
/// a client — the server build is an ordinary host binary. **The rendered
/// tree is not gated**, and must not be: the server renders the HTML the WASM
/// then hydrates, so a tree that differs between the two breaks hydration in
/// ways that surface as unrelated event handlers going dead. Gate the call,
/// never the markup.
///
/// Within the client targets nothing is gated, because a plugin with no
/// implementation for the current platform compiles to an inert no-op rather
/// than a build error. "Compiles everywhere" is not "works everywhere",
/// though: check the table in docs/native-plugins.md before relying on one.
#[component]
fn NativePluginDemos() -> Element {
    #[cfg(any(
        all(feature = "web", target_arch = "wasm32"),
        target_os = "android",
        target_os = "ios"
    ))]
    let toast = use_toast();

    #[cfg(any(
        all(feature = "web", target_arch = "wasm32"),
        target_os = "android",
        target_os = "ios"
    ))]
    let mut plugins = use_context::<g3_native_plugins::NativePlugins>();

    rsx! {
        Card { title: "Native plugins",
            Text { tone: TextTone::Secondary, "The same calls on web, Android, and iOS." }
            List { variant: ListVariant::EdgeToEdge, lines: ListLines::Inset,
                Item {
                    start: rsx! { ClipboardCopy { size: 20 } },
                    label: "Copy text",
                    description: "To the system clipboard",
                    detail: ItemDetail::Hide,
                    onclick: move |_| {
                        cfg_if::cfg_if! { if #[cfg(any(all(feature = "web", target_arch = "wasm32"), target_os = "android", target_os = "ios"))] {
                            match plugins.clipboard.write().copy_to_clipboard("Sent from the g3 stack".to_string()) {
                                Ok(()) => { toast.success("Copied to clipboard."); }
                                Err(error) => { toast.error(format!("Clipboard unavailable: {error}")); }
                            }
                        }}
                    },
                }
                Item {
                    start: rsx! { Share2 { size: 20 } },
                    label: "Share",
                    description: "Opens the share sheet",
                    detail: ItemDetail::Hide,
                    onclick: move |_| {
                        cfg_if::cfg_if! { if #[cfg(any(all(feature = "web", target_arch = "wasm32"), target_os = "android", target_os = "ios"))] {
                            // Browsers without Web Share return an error rather
                            // than doing nothing, so the app can say so.
                            if let Err(error) = plugins.clipboard.write().share("Built on the g3 stack".to_string()) {
                                toast.error(format!("Sharing unavailable here: {error}"));
                            }
                        }}
                    },
                }
                Item {
                    start: rsx! { ExternalLink { size: 20 } },
                    label: "Open the Dioxus docs",
                    description: "In the system browser",
                    detail: ItemDetail::Hide,
                    onclick: move |_| {
                        cfg_if::cfg_if! { if #[cfg(any(all(feature = "web", target_arch = "wasm32"), target_os = "android", target_os = "ios"))] {
                            if let Err(error) = plugins.external_url.write().open("https://dioxuslabs.com/learn/0.7/") {
                                toast.error(format!("Could not open the browser: {error}"));
                            }
                        }}
                    },
                }
            }
        }
    }
}
