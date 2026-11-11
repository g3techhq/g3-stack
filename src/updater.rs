//! Where a sideloaded build looks for a newer version of the app, and the key
//! a release must be signed with (g3-native-plugins' `updater`).
//!
//! Releases go to g3techhq/releases, the repository every g3 app shares, and
//! the app reads `apps/<crate name>/latest.json` there. Only the `sideload`
//! build (a debug-signed APK installed by hand) turns the updater on; web,
//! server and store builds never compile it. See docs/deployment.md,
//! "In-app updates".
#![cfg_attr(
    not(feature = "sideload"),
    allow(
        dead_code,
        reason = "Only the sideloaded build reads these; the tests check them in every build."
    )
)]

/// The app's directory in g3techhq/releases: the crate name, as
/// `scripts/publish-dev.mjs` publishes it.
pub const APP_ID: &str = env!("CARGO_PKG_NAME");

pub const RELEASES: &str = concat!(
    "https://raw.githubusercontent.com/g3techhq/releases/main/apps/",
    env!("CARGO_PKG_NAME"),
    "/latest.json"
);

/// The public half of the app's release key, both lines. Generate the pair
/// once, in a checkout of g3techhq/releases:
///
/// ```sh
/// node scripts/release-signing.mjs keygen ~/.g3-release <crate name>
/// ```
///
/// then paste the contents of `<crate name>-updater.pub` here. The secret key
/// stays in `~/.g3-release`, never in a repository. Until then the updater
/// stays off, and `publish-dev` refuses to publish.
pub const UPDATER_PUBLIC_KEY: &str =
    "untrusted comment: PLACEHOLDER, not a key yet (see the comment above)
";

/// What marks [`UPDATER_PUBLIC_KEY`] as not yet replaced.
const PLACEHOLDER: &str = "PLACEHOLDER";

/// Whether a real key has been pasted in.
pub fn has_public_key() -> bool {
    !UPDATER_PUBLIC_KEY.contains(PLACEHOLDER)
}

/// Before launch, so the prompt and Settings find the updater ready. Does
/// nothing outside the `sideload` build, and logs and leaves the updater off
/// while the key is still the placeholder.
pub fn configure() {
    #[cfg(feature = "sideload")]
    {
        use g3_native_plugins::{Updater, UpdaterConfig};

        // `eprintln!`: this runs before `launch` sets up the logger.
        if !has_public_key() {
            eprintln!(
                "The {APP_ID} updater is off: src/updater.rs still has the placeholder release key."
            );
            return;
        }
        let config =
            UpdaterConfig::new(UPDATER_PUBLIC_KEY, env!("CARGO_PKG_VERSION")).endpoint(RELEASES);
        if let Err(error) = Updater::new().configure(config) {
            // The key and version are compiled in, so this is a build mistake.
            eprintln!("The {APP_ID} updater is misconfigured: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn releases_are_read_from_the_apps_own_directory() {
        assert_eq!(
            RELEASES,
            format!(
                "https://raw.githubusercontent.com/g3techhq/releases/main/apps/{APP_ID}/latest.json"
            )
        );
    }

    #[test]
    fn the_key_is_pasted_in_whole() {
        // Both lines of the `.pub` file: the comment, then the key. Only the
        // placeholder has no second line.
        assert!(UPDATER_PUBLIC_KEY.starts_with("untrusted comment:"));
        if has_public_key() {
            assert_eq!(UPDATER_PUBLIC_KEY.trim().lines().count(), 2);
        }
    }

    #[cfg(feature = "sideload")]
    #[test]
    fn the_compiled_in_key_and_version_are_accepted() {
        use g3_native_plugins::{Updater, UpdaterConfig};

        if !has_public_key() {
            return;
        }
        let config = UpdaterConfig::new(UPDATER_PUBLIC_KEY, env!("CARGO_PKG_VERSION"));
        assert!(Updater::new().configure(config).is_ok());
    }
}
