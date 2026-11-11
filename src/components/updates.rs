//! In-app updates for the sideloaded build: a prompt at launch, and a card in
//! Settings.
//!
//! Both render nothing outside the `sideload` build, and nothing there either
//! until `crate::updater::configure` has turned the updater on (it stays off
//! while the release key is the placeholder). Web, server and store builds
//! therefore render the same empty tree, so a server render and the page it
//! hydrates still match.
//!
//! g3tech apps that depend on g3-brand can use its `updater` feature
//! (`UpdatePrompt`, `AppUpdates`) instead; see docs/g3-brand.md.

#[cfg(not(feature = "sideload"))]
use dioxus::prelude::*;

/// Checks for a release when the app starts and offers to download and
/// install it, through `use_alert`. Mount once, inside `AppWrapper`.
#[cfg(not(feature = "sideload"))]
#[component]
pub fn UpdatePrompt() -> Element {
    rsx! {}
}

/// The Settings card: the running version, the updater's state, and the
/// button that moves it on.
#[cfg(not(feature = "sideload"))]
#[component]
pub fn AppUpdates() -> Element {
    rsx! {}
}

#[cfg(feature = "sideload")]
pub use native::{AppUpdates, UpdatePrompt};

#[cfg(feature = "sideload")]
mod native {
    use dioxus::prelude::*;
    use dioxus_icons::lucide::{Download, RefreshCw};
    use g3_native_plugins::{Updater, UpdaterState};
    use g3_ui::{
        AlertButton, AlertOptions, Alerts, Button, ButtonFill, ButtonSize, Card, Color, Item, List,
        ListVariant, Space, Stack, Text, TextVariant, use_alert, use_toast,
    };

    /// Whether `configure` turned the updater on: it only records a version
    /// once the key and version are accepted.
    fn configured() -> bool {
        Updater::new().current_version().is_some()
    }

    /// Polls the updater, which reports a state rather than events, while
    /// the component is mounted. A page timer paces it, since no Rust timer
    /// is shared by every build; `id` keeps two pollers apart.
    fn use_updater_state(id: &'static str) -> Signal<UpdaterState> {
        let mut state = use_signal(|| Updater::new().state());
        use_future(move || async move {
            if !configured() {
                return;
            }
            let mut tick = document::eval(&format!(
                "clearInterval(window.{id}); window.{id} = setInterval(() => dioxus.send(true), 500);"
            ));
            while tick.recv::<bool>().await.is_ok() {
                let next = Updater::new().state();
                if *state.peek() != next {
                    state.set(next);
                }
            }
        });
        use_drop(move || {
            let _ = document::eval(&format!("clearInterval(window.{id});"));
        });
        state
    }

    /// Asks with the app's own alert; `true` when `yes` was pressed.
    async fn ask(alerts: Alerts, title: &str, message: String, yes: &str) -> bool {
        let buttons = vec![AlertButton::cancel("Later"), AlertButton::new(yes)];
        alerts
            .show(AlertOptions {
                title: title.to_string(),
                message: Some(message),
                buttons: buttons.clone(),
                input: None,
            })
            .await
            .confirmed(&buttons)
    }

    #[component]
    pub fn UpdatePrompt() -> Element {
        let alerts = use_alert();
        let toast = use_toast();
        let state = use_updater_state("__g3UpdatePromptTick");
        // The step already offered, per version, so a poll does not ask
        // again. "Later" stops offering for the rest of this launch.
        let mut offered = use_signal(|| None::<(String, &'static str)>);
        let mut declined = use_signal(|| false);
        let mut downloading = use_signal(|| false);

        use_hook(|| {
            if configured() && Updater::new().state() == UpdaterState::Idle {
                // A failed check stays quiet: Settings shows it.
                let _ = Updater::new().start_check();
            }
        });

        use_effect(move || {
            let current = state();
            if declined() {
                return;
            }
            let (update, step) = match &current {
                UpdaterState::Available(update) if !update.store => (update.clone(), "download"),
                UpdaterState::Ready(update) => (update.clone(), "install"),
                UpdaterState::NeedsPermission(update) => (update.clone(), "permission"),
                UpdaterState::Failed(error) if *downloading.peek() => {
                    downloading.set(false);
                    toast.error(format!("Could not update: {error}"));
                    return;
                }
                _ => return,
            };
            let key = (update.version.clone(), step);
            if offered.peek().as_ref() == Some(&key) {
                return;
            }
            offered.set(Some(key));
            spawn(async move {
                let version = update.version;
                let (title, message, yes) = match step {
                    "download" => (
                        "Update available",
                        format!("Version {version} is out. Download it now?"),
                        "Download",
                    ),
                    "install" => (
                        "Update downloaded",
                        format!("Version {version} is downloaded and verified."),
                        if cfg!(target_os = "android") {
                            "Install"
                        } else {
                            "Restart and install"
                        },
                    ),
                    _ => (
                        "Allow installing apps",
                        "Allow this app to install apps in the screen that opened, then come back and install."
                            .to_string(),
                        "Install",
                    ),
                };
                if !ask(alerts, title, message, yes).await {
                    declined.set(true);
                    return;
                }
                let mut updater = Updater::new();
                let started = if step == "download" {
                    downloading.set(true);
                    updater.start_download()
                } else {
                    updater.install()
                };
                if let Err(error) = started {
                    toast.error(format!("Could not update: {error}"));
                }
            });
        });

        rsx! {}
    }

    #[component]
    pub fn AppUpdates() -> Element {
        let toast = use_toast();
        let mut state = use_updater_state("__g3AppUpdatesTick");

        if !configured() {
            return rsx! {};
        }

        let current = env!("CARGO_PKG_VERSION");
        let (description, action) = describe(&state.read());

        rsx! {
            Card { title: "App updates",
                Stack { gap: Space::Md,
                    List { variant: ListVariant::Filled,
                        Item {
                            wrap: true,
                            start: rsx! { RefreshCw { size: 18 } },
                            label: "Version {current}",
                            description: "{description}",
                        }
                    }
                    if let Some(action) = action {
                        Stack { horizontal: true, gap: Space::Sm,
                            Button {
                                fill: if action == Action::Check { ButtonFill::Outline } else { ButtonFill::Solid },
                                color: if action == Action::Check { Color::Neutral } else { Color::Accent },
                                size: ButtonSize::Sm,
                                start: rsx! { Download { size: 16 } },
                                onclick: move |_| {
                                    let mut updater = Updater::new();
                                    let started = match action {
                                        Action::Check => updater.start_check(),
                                        Action::Download => updater.start_download(),
                                        // A store page opens without a download first.
                                        Action::Install | Action::OpenStore => updater.install(),
                                    };
                                    match started {
                                        Ok(()) => state.set(updater.state()),
                                        Err(error) => {
                                            toast.error(format!("Could not update: {error}"));
                                        }
                                    }
                                },
                                "{action.label()}"
                            }
                        }
                    }
                    Text { variant: TextVariant::Caption,
                        "Releases come from github.com/g3techhq/releases and are checked against this app's signing key before they install."
                    }
                }
            }
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Action {
        Check,
        Download,
        Install,
        OpenStore,
    }

    impl Action {
        fn label(self) -> &'static str {
            match self {
                Action::Check => "Check for updates",
                Action::Download => "Download",
                Action::OpenStore => "Open the store",
                // Desktop closes and reopens; Android may show the system's
                // own confirmation.
                Action::Install => {
                    if cfg!(target_os = "android") {
                        "Install"
                    } else {
                        "Restart and install"
                    }
                }
            }
        }
    }

    /// What to say about the updater's state, and the button that moves it on.
    fn describe(state: &UpdaterState) -> (String, Option<Action>) {
        match state {
            UpdaterState::Idle => (
                "Check GitHub for a newer release".to_string(),
                Some(Action::Check),
            ),
            UpdaterState::Checking => ("Checking for updates\u{2026}".to_string(), None),
            UpdaterState::UpToDate => (
                "This is the latest version".to_string(),
                Some(Action::Check),
            ),
            UpdaterState::Available(update) if update.store => (
                format!("Version {} is in the store", update.version),
                Some(Action::OpenStore),
            ),
            UpdaterState::Available(update) => (
                format!("Version {} is available", update.version),
                Some(Action::Download),
            ),
            UpdaterState::Downloading { update, progress } => {
                let text = match progress.bytes_total {
                    Some(total) if total > 0 => format!(
                        "Downloading {}: {}%",
                        update.version,
                        progress.bytes_done.saturating_mul(100) / total
                    ),
                    _ => format!(
                        "Downloading {}: {}",
                        update.version,
                        megabytes(progress.bytes_done)
                    ),
                };
                (text, None)
            }
            UpdaterState::Ready(update) => (
                format!("Version {} is downloaded and verified", update.version),
                Some(Action::Install),
            ),
            UpdaterState::NeedsPermission(_) => (
                "Allow this app to install apps in the screen that opened, then come back and install"
                    .to_string(),
                Some(Action::Install),
            ),
            UpdaterState::Installing(update) => {
                (format!("Installing {}\u{2026}", update.version), None)
            }
            UpdaterState::Failed(error) => {
                (format!("Could not update: {error}"), Some(Action::Check))
            }
        }
    }

    fn megabytes(bytes: u64) -> String {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use g3_native_plugins::{DownloadProgress, Update};

        fn update() -> Update {
            Update {
                version: "0.2.0".to_string(),
                current_version: "0.1.0".to_string(),
                notes: None,
                pub_date: None,
                store: false,
            }
        }

        #[test]
        fn each_state_offers_the_step_that_moves_it_on() {
            assert_eq!(describe(&UpdaterState::Idle).1, Some(Action::Check));
            assert_eq!(describe(&UpdaterState::Checking).1, None);
            let (text, action) = describe(&UpdaterState::Available(update()));
            assert_eq!(text, "Version 0.2.0 is available");
            assert_eq!(action, Some(Action::Download));
            let store = Update {
                store: true,
                ..update()
            };
            assert_eq!(
                describe(&UpdaterState::Available(store)).1,
                Some(Action::OpenStore)
            );
            let downloading = UpdaterState::Downloading {
                update: update(),
                progress: DownloadProgress {
                    bytes_done: 25,
                    bytes_total: Some(100),
                },
            };
            assert_eq!(
                describe(&downloading),
                ("Downloading 0.2.0: 25%".to_string(), None)
            );
            let unknown = UpdaterState::Downloading {
                update: update(),
                progress: DownloadProgress {
                    bytes_done: 2_500_000,
                    bytes_total: None,
                },
            };
            assert_eq!(describe(&unknown).0, "Downloading 0.2.0: 2.5 MB");
            assert_eq!(
                describe(&UpdaterState::Ready(update())).1,
                Some(Action::Install)
            );
            assert_eq!(
                describe(&UpdaterState::NeedsPermission(update())).1,
                Some(Action::Install)
            );
            assert_eq!(
                describe(&UpdaterState::Failed("offline".to_string())).1,
                Some(Action::Check)
            );
        }
    }
}
