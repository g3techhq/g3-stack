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
`dioxus-cookie` (enabled by the `mobile` feature) stores them in the Keychain on
iOS and the Keystore on Android, and `main` calls `dioxus_cookie::init()` before
launch. Without it, sign-in succeeds and every following call is a 401.

On a simulator or emulator whose keychain is unavailable, the `mobile-sim`
feature falls back to an encrypted file. That fallback is compiled out of release
builds.

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
  `g3_ui::init_auto_mode()` and `g3_route_transitions::init_auto_platform()`
  and drop the stored preference.
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

Before the first store submission:

- Set `[bundle] identifier`, `publisher`, and the icon in `Dioxus.toml`.
- Both stores require **in-app account deletion** for any app with accounts.
  The template ships it: Settings → Delete account, backed by `delete_account`
  in `src/auth/account.rs`.
- Both stores require a privacy policy URL, and Apple requires Sign in with
  Apple if you offer any other third-party sign-in.
