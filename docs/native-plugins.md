# Native plugins

[`g3-native-plugins`](https://github.com/g3techhq/g3-native-plugins) is how a
Dioxus app reaches what a WebView cannot: the share sheet, the keychain, Sign in
with Apple, system Back. One call site serves web, Android, and iOS.

---

## Enabling a plugin

Each plugin is a Cargo feature. The template enables three:

```toml
g3-native-plugins = { version = "0.4.2", features = ["back-button", "clipboard", "external-url"] }
```

| Feature | Android | iOS | Web | What it does |
| --- | --- | --- | --- | --- |
| `back-button` | yes | yes | — | System Back and the iOS edge swipe. Wired to navigation already; see [navigation.md](navigation.md) |
| `clipboard` | yes | yes | yes | Copy, and the native share sheet (Web Share in a browser) |
| `external-url` | yes | yes | yes | Open a URL in the system browser |
| `storage` | yes | yes | yes\* | Key-value storage in the Keystore / Keychain. \*`localStorage` on web, **unencrypted** |
| `auth` | yes | yes | — | Google Sign-In (Android), Sign in with Apple (iOS) |
| `deep-links` | yes | yes | — | Receive the URL a universal link or App Link opened the app with |
| `geolocation` | yes | yes | — | Position requests and the permission prompt. On web, use `navigator.geolocation` |
| `camera-microphone` | yes | yes | — | Permission state and prompts around `getUserMedia` |
| `media` | yes | yes | — | Background playback, lock-screen controls, picture-in-picture |
| `in-app-purchases` | yes | yes | — | StoreKit 2 and Play Billing: products, purchases, subscriptions, restore |
| `notifications` | yes | yes | — | Local notifications, shown now or scheduled, with buttons, replies and Android channels |
| `push-notifications` | yes | yes | — | APNs and Firebase Cloud Messaging: tokens, messages and taps. Needs a Firebase project on Android |
| `updater` | yes | yes | — | Signed over-the-air updates of the web bundle the WebView loads (never the Rust binary) |

"—" means the calls compile and do nothing. **A plugin never fails the build on
a platform it does not support**, which is what lets one dependency line serve
every target — and also means an unsupported platform shows up as nothing
happening, not as an error. Check this table first.

The crate README has the details for each: required `Dioxus.toml` permissions,
provisioning, and the platform differences each one surfaces.

---

## Calling one

`NativePluginsProvider` is already mounted in `src/app.rs`. Get the handle from
context and call through it:

```rust
let mut plugins = use_context::<g3_native_plugins::NativePlugins>();
plugins.clipboard.write().copy_to_clipboard("Hello".to_string())?;
```

### The cfg pattern, and why it matters

`NativePlugins` only exists on targets with a client. The **server** build is an
ordinary host binary and has no such context, yet it renders every component to
HTML. So the context lookup and the call are gated; **the markup is not**:

```rust
#[component]
fn ShareButton() -> Element {
    #[cfg(any(all(feature = "web", target_arch = "wasm32"), target_os = "android", target_os = "ios"))]
    let mut plugins = use_context::<g3_native_plugins::NativePlugins>();

    rsx! {
        Button {
            onclick: move |_| {
                cfg_if::cfg_if! { if #[cfg(any(all(feature = "web", target_arch = "wasm32"), target_os = "android", target_os = "ios"))] {
                    let _ = plugins.clipboard.write().share("Built on the g3 stack".to_string());
                }}
            },
            "Share"
        }
    }
}
```

Never wrap the `rsx!` itself in `cfg`. The server renders HTML that the WASM
client then hydrates, and a tree that differs between the two breaks hydration
in a way that surfaces as unrelated event handlers silently doing nothing.

`NativePluginDemos` in `src/components/settings.rs` is a working example with
three plugins.

### Results arrive by polling

Plugins whose answer comes back later — sign-in, location, purchases — use
start-and-poll rather than `async`:

```rust
let challenge = prepare_apple_signin(true, None).await?;
plugins
    .auth
    .write()
    .start_apple_auth(&challenge.state, &challenge.nonce)?;

use_future(move || async move {
    loop {
        if let Ok(Some(credential)) = plugins.auth.write().poll_auth_result() {
            // Consume the stored challenge, compare state, verify the hashed
            // nonce in the ID token, and exchange authorization_code.
            break;
        }
        if !plugins.auth.write().is_auth_awaiting() {
            break; // cancelled
        }
        // Any async sleep works here; `tokio::time::sleep` is already in the
        // mobile build. `gloo-timers` is the usual choice for a web one.
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
});
```

The native side delivers on a platform callback that a blocked Dioxus task
would never see, and a poll loop keeps the UI responsive meanwhile.

---

## Adding one

1. Add the feature to `g3-native-plugins` in `Cargo.toml`.
2. Declare any permission it needs in `Dioxus.toml` (`[permissions]`,
   `[ios]`, `[android]`) — the crate README says which. A missing declaration
   usually fails silently on Android and terminates the app on iOS.
3. Call it with the cfg pattern above.
4. Check `just check`, which builds the server feature set too: a plugin call
   that is not gated is a server compile error.
5. Test on a device or emulator. The web build tells you nothing about the
   native implementation.
