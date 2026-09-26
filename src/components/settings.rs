use crate::{app::Route, auth::delete_account, db::ColorScheme, state::AppState};
use dioxus::prelude::*;
use dioxus_icons::lucide::{ClipboardCopy, ExternalLink, Share2};
use g3_ui::{
    Button, ButtonStyle, Card, ComponentMode, ConfirmModal, Item, ItemKind, List, ListLines,
    SegmentButton, SegmentGroup, StatusColor, Toggle,
};

/// Appearance, native-plugin demos, and the account.
///
/// The appearance controls are the interesting part: they drive both g3-ui's
/// component mode and g3-route-transitions' motion language from one place
/// (see `AppState::set_appearance`), so switching to Material restyles every
/// component *and* changes how the next navigation animates.
#[component]
pub fn Settings() -> Element {
    let mut app_state = use_context::<AppState>();
    let navigator = use_navigator();
    let mode = (app_state.mode)();
    let scheme = (app_state.color_scheme)();

    // `SegmentGroup` and `Toggle` own a `Signal`, so these mirror `AppState`
    // rather than reading it directly.
    let mut mode_index = use_signal(|| mode_to_index(mode));
    let mut dark = use_signal(|| scheme == ColorScheme::Dark);
    let mut confirm_delete = use_signal(|| false);

    // `use_signal`'s initializer runs once, so a mirror set up before the
    // stored preference arrived would keep showing the default while the rest
    // of the app had already switched. These re-sync it when `AppState`
    // changes for a reason other than this screen's own controls.
    use_effect(use_reactive!(|mode| mode_index.set(mode_to_index(mode))));
    use_effect(use_reactive!(|scheme| dark.set(scheme == ColorScheme::Dark)));

    let handle = app_state
        .user
        .read()
        .as_ref()
        .map(|user| user.handle.clone())
        .unwrap_or_default();

    rsx! {
        Card { title: "Appearance",
            p { class: "g3-message-text g3-message-text-muted",
                "iOS and Material are Ionic's two modes. Switching restyles every component "
                "and changes how the next screen animates in."
            }
            SegmentGroup {
                active: mode_index,
                on_change: move |index: usize| {
                    app_state.set_appearance(index_to_mode(index), (app_state.color_scheme)());
                },
                SegmentButton { index: 0, "iOS" }
                SegmentButton { index: 1, "Material" }
            }
            List { inset: true, lines: ListLines::None,
                Item {
                    label: "Dark mode",
                    end: rsx! {
                        Toggle {
                            checked: dark,
                            onchange: move |on: bool| {
                                let scheme = if on { ColorScheme::Dark } else { ColorScheme::Light };
                                app_state.set_appearance((app_state.mode)(), scheme);
                            },
                        }
                    },
                }
            }
        }

        NativePluginDemos {}

        Card { title: "Account",
            List { inset: true, lines: ListLines::Inset,
                Item { label: "Signed in as", metadata: handle }
                Item {
                    kind: ItemKind::Button,
                    label: "Sign out",
                    onclick: move |_| app_state.sign_out_confirm_open.set(true),
                }
            }
        }

        Button {
            style: ButtonStyle::Danger,
            expand: true,
            onclick: move |_| confirm_delete.set(true),
            "Delete account"
        }

        ConfirmModal {
            open: confirm_delete,
            title: "Delete your account?",
            description: rsx! { "Your account and every note in it are removed for good. This cannot be undone." },
            confirm_text: "Delete account",
            on_confirm: move |_| {
                spawn(async move {
                    match delete_account().await {
                        Ok(()) => {
                            app_state.user.set(None);
                            app_state.show_toast("Account deleted.", StatusColor::Neutral);
                            // `replace`: the page this entry points at belonged
                            // to an account that no longer exists.
                            navigator.replace(Route::Splash {});
                        }
                        Err(error) => app_state.show_toast(
                            format!("Could not delete the account: {error}"),
                            StatusColor::Danger,
                        ),
                    }
                });
            },
        }
    }
}

fn mode_to_index(mode: ComponentMode) -> usize {
    match mode {
        ComponentMode::Ios => 0,
        ComponentMode::Md => 1,
    }
}

fn index_to_mode(index: usize) -> ComponentMode {
    if index == 1 {
        ComponentMode::Md
    } else {
        ComponentMode::Ios
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
    let mut app_state = use_context::<AppState>();

    #[cfg(any(
        all(feature = "web", target_arch = "wasm32"),
        target_os = "android",
        target_os = "ios"
    ))]
    let mut plugins = use_context::<g3_native_plugins::NativePlugins>();

    rsx! {
        Card { title: "Native plugins",
            p { class: "g3-message-text g3-message-text-muted",
                "The same calls on web, Android, and iOS."
            }
            List { inset: true, lines: ListLines::Inset,
                Item {
                    kind: ItemKind::Button,
                    start: rsx! { ClipboardCopy { size: 20 } },
                    label: "Copy text",
                    description: "To the system clipboard",
                    onclick: move |_| {
                        cfg_if::cfg_if! { if #[cfg(any(all(feature = "web", target_arch = "wasm32"), target_os = "android", target_os = "ios"))] {
                            match plugins.clipboard.write().copy_to_clipboard("Sent from the g3 stack".to_string()) {
                                Ok(()) => app_state.show_toast("Copied to clipboard.", StatusColor::Success),
                                Err(error) => app_state.show_toast(
                                    format!("Clipboard unavailable: {error}"),
                                    StatusColor::Warning,
                                ),
                            }
                        }}
                    },
                }
                Item {
                    kind: ItemKind::Button,
                    start: rsx! { Share2 { size: 20 } },
                    label: "Share",
                    description: "Opens the share sheet",
                    onclick: move |_| {
                        cfg_if::cfg_if! { if #[cfg(any(all(feature = "web", target_arch = "wasm32"), target_os = "android", target_os = "ios"))] {
                            // Browsers without Web Share return an error rather
                            // than doing nothing, so the app can say so.
                            if let Err(error) = plugins.clipboard.write().share("Built on the g3 stack".to_string()) {
                                app_state.show_toast(
                                    format!("Sharing unavailable here: {error}"),
                                    StatusColor::Warning,
                                );
                            }
                        }}
                    },
                }
                Item {
                    kind: ItemKind::Button,
                    start: rsx! { ExternalLink { size: 20 } },
                    label: "Open the Dioxus docs",
                    description: "In the system browser",
                    onclick: move |_| {
                        cfg_if::cfg_if! { if #[cfg(any(all(feature = "web", target_arch = "wasm32"), target_os = "android", target_os = "ios"))] {
                            if let Err(error) = plugins.external_url.write().open("https://dioxuslabs.com/learn/0.7/") {
                                app_state.show_toast(
                                    format!("Could not open the browser: {error}"),
                                    StatusColor::Warning,
                                );
                            }
                        }}
                    },
                }
            }
        }
    }
}
