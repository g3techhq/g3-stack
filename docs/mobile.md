# Android and iOS

The same `src/` runs on phones, in a native WebView, with the `mobile` feature.
The server is the same server: a phone calls your server functions over HTTPS.

---

## Setup

Follow the Dioxus guide for the platform toolchains:
[dioxuslabs.com/learn/0.7/guides/platforms/mobile](https://dioxuslabs.com/learn/0.7/guides/platforms/mobile).

- **Android:** Android Studio, an SDK and NDK, and `rustup target add
  aarch64-linux-android x86_64-linux-android`. Set `ANDROID_HOME` and
  `ANDROID_NDK_HOME`.
- **iOS:** macOS, Xcode, and `rustup target add aarch64-apple-ios
  aarch64-apple-ios-sim`.

## Running

Start the database, then:

```bash
just dev-android   # an emulator or a connected device
just dev-ios       # the iOS simulator
```

`dx serve --platform android|ios` runs the fullstack server on your machine and
points the app at it, so in development there is nothing to configure.

---

## How a phone talks to the server

### SERVER_URL

A web page calls the server that served it. An installed app has no such
origin, so its server has to be compiled in. `src/server_url.rs` handles it:

| Build | Where calls go |
| --- | --- |
| Web, any | The page's own origin. `SERVER_URL` is ignored |
| Mobile, debug, `SERVER_URL` unset | The `dx serve` dev server, found automatically |
| Mobile, debug, `SERVER_URL` set | That URL. HTTP is allowed for `localhost` only |
| Mobile, release | `SERVER_URL`, which **must** be set and **must** be HTTPS, or the build panics at startup |

`SERVER_URL` is read at compile time:

```bash
SERVER_URL=https://app.example.com dx bundle --platform android --release
```

Validation rejects anything but a bare origin — no path, query, or credentials —
because server functions live at `/api/..` on it, and a path would be prepended
to every call.

### Cookies

The session is a cookie. A browser keeps cookies; a native HTTP client does not.
g3-auth's `mobile` feature (enabled by this app's `mobile` feature) stores them in
the Keychain on iOS and the Keystore on Android, and `main` calls
`g3_auth::init()` before launch. Without it, sign-in succeeds and every following
call is a 401, and a relaunch signs everyone out.

On a simulator or emulator whose keychain is unavailable it falls back to an
encrypted file. That fallback is compiled out of release builds.

### Signed-out launches

On the web, a signed-out page load is redirected by the server to the splash. A
native app makes no page load: it starts its router at `/`. That is why the
splash *is* `/` — both paths arrive at the same screen that asks the server once
whether there is a session. See [authentication.md](authentication.md).

---

## Platform feel

- **Back.** Android's Back gesture and iOS's left-edge swipe run the same
  animated pop as the in-app back button, and do the platform's normal thing at
  the root. See [navigation.md](navigation.md).
- **Mode.** The template defaults to iOS styling everywhere and lets the user
  switch in Settings. To follow the device instead, call
  `g3_ui::init_auto_mode()`, leave g3-route-transitions' platform unset (its
  `platform()` falls back to `detect_platform()`), and drop the stored
  preference.
- **Safe areas and the keyboard.** g3-ui's shell pads for notches and the home
  indicator. Keep content inside `Body` and it is handled.

---

## Deep links

Opening `https://app.example.com/notes/abc` on a phone with the app installed
can open the app on that note. Three pieces:

1. **Declare the domain.** `Dioxus.toml`:

   ```toml
   [deep_links]
   hosts = ["app.example.com"]
   paths = ["/notes/*"]
   ```

2. **Prove you own it.** Serve `/.well-known/apple-app-site-association` and
   `/.well-known/assetlinks.json` from that domain. `g3-native-plugins` has a
   macro for each; add them to the server build. The auth guard already lets
   `/.well-known/` through without a session:

   ```rust
   g3_native_plugins::ios_app_site_association_route! {
       team_id: "TEAMID",
       bundle_id: "com.example.g3app",
       paths: ["/notes/*"],
   }
   g3_native_plugins::android_asset_links_route! {
       package_name: "com.example.g3app",
       sha256_cert_fingerprints: ["AA:BB:.."],
   }
   ```

3. **Receive the URL.** Enable the `deep-links` feature, call
   `DeepLinks::new().prepare()` in `main` before launch, and drain
   `plugins.deep_links.write().take_link()` somewhere that can navigate. The
   crate README covers the cold-start and Android launch-mode details.

---

## Releasing

```bash
SERVER_URL=https://app.example.com dx bundle --platform android --release
SERVER_URL=https://app.example.com dx bundle --platform ios --release
```

The **Build Mobile Artifacts** workflow (`.github/workflows/build-mobile.yml`)
does the same on CI, with what a store upload needs on top. The store-release
fixes it relies on (`versionCode`, `--appstore`, icons, signed Swift
frameworks) are in the g3 fork's `dx`, not in 0.7.9.

- **Build numbers.** Every upload needs a higher one than the last. The
  workflow passes its run number as `DX_ANDROID_VERSION_CODE` (Android's
  `versionCode`) and `DX_BUNDLE_VERSION` (iOS's `CFBundleVersion`); locally,
  set them yourself or use `--version-code` / `--bundle-version`.
- **Android signing.** `dx` signs with a keystore named under
  `[android.signing]` in `Dioxus.toml` (`jks_file`, `jks_password`,
  `key_alias`, `key_password`), as plain values, so never commit that table.
  The workflow adds it on the runner when the `ANDROID_UPLOAD_KEYSTORE_BASE64`,
  `ANDROID_UPLOAD_KEYSTORE_PASSWORD`, `ANDROID_UPLOAD_KEY_ALIAS` and
  `ANDROID_UPLOAD_KEY_PASSWORD` secrets exist. Use Play App Signing with an
  upload key. The Gradle project `dx` generates holds the passwords too, so
  never upload or commit `target/`.
- **iOS signing.** With the `IOS_DISTRIBUTION_CERTIFICATE_BASE64` (a `.p12`),
  `IOS_DISTRIBUTION_CERTIFICATE_PASSWORD` and `IOS_APPSTORE_PROFILE_BASE64`
  secrets, the workflow installs an Apple Distribution certificate and an App
  Store provisioning profile on the runner and bundles with `--appstore`.
  Without them both platforms build unsigned, as before.

Before the first store submission:

- Set `[bundle] identifier` (lowercase reverse DNS; it can never change once
  published) and `publisher` in `Dioxus.toml`, and replace the icon: draw a
  full-bleed `assets/icon/icon.svg` with the mark inside the centre 62%, then
  `npm run icon -- assets/icon/icon.svg`. dx renders the Android launcher
  icons from the SVG (`[bundle] icon`); the script writes `icon-1024.png`,
  the opaque RGB icon `[ios] icon` names, and `icon-512.png` for the Play
  listing.
- `ios/PrivacyInfo.xcprivacy` declares the required-reason APIs that Rust's
  standard library and tokio use. Add any your own code or crates call.
- `[ios.plist] ITSAppUsesNonExemptEncryption = false` answers the export
  compliance question for an app that only uses HTTPS. Set it to `true` if you
  add encryption of your own.
- Check `[android] target_sdk` against Google Play's current target API rule,
  and the iOS job's Xcode against what App Store Connect requires. The workflow
  runs on `macos-26` and stops unless Xcode is 26 or newer; raise both when
  Apple raises the SDK rule.
- Both stores require **in-app account deletion** for any app with accounts.
  The template ships it: Settings → Delete account, backed by `delete_account`
  in `src/auth/account.rs`.
- Both stores require a privacy policy URL, and Apple requires Sign in with
  Apple if you offer any other third-party sign-in.
