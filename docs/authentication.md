# Authentication

Sessions, the auth guard, sign-in, and account deletion. The code is in
`src/auth/`.

---

## What ships

- **Guest sign-in** (`src/auth/guest.rs`). One tap creates a real `user` row and
  a real session. Every screen past sign-in can assume a signed-in user; there
  is no half-signed-in state to design around.
- **Session cookies** stored in SurrealDB (`axum_session`, with the store in
  `src/auth/session_store.rs`), resolved to a `SessionUser` on each request by
  `axum_session_auth`.
- **A guard** that rejects unauthenticated requests by default.
- **Sign-out** and **account deletion**, both from Settings.

## The pieces, in request order

```
request
  → SessionLayer         reads the cookie, loads the session row
  → AuthSessionLayer     resolves it to a SessionUser
  → auth_check           401 or redirect, unless the path is allowlisted
  → your server function StateExtractor { db, session_user, auth_session }
```

### Guarded by default

`auth_check` in `src/auth/session.rs` rejects every path that is not in
`is_unsecured_path`. A server function you add is protected without you doing
anything. Opening one up is a deliberate edit to that list, and a test asserts
that the app's own endpoints are not on it.

Add a path only when it genuinely has to answer a visitor with no session: a
sign-in endpoint, an OAuth callback, a public share link, a legal page a store
listing links to, the `.well-known` files for deep links.

### 401 for a fetch, redirect for a page

A signed-out **page load** is redirected to the splash. A signed-out **server
function call** gets a `401` with a JSON body shaped like Dioxus's own error.

The split matters. `fetch` follows redirects silently, so a server function
redirected to an HTML page would receive markup where it expected JSON and fail
with an opaque decode error — the failure you would otherwise see every time the
dev server restarts and the cookie's session no longer exists.

### One place decides

The splash (`/`, `src/components/auth/splash.rs`) asks `is_signed_in` once and
navigates to the app or to sign-in. Nothing else in the client makes that
decision. It is `/` so that web and native agree: the web gets there by
redirect, a native app starts there.

A second guard — a layout that checks `AppState.user`, say — would be a second
source of truth about the same question. The day the two disagree, a signed-in
user is thrown out of the app.

---

## Writing a server function that uses the session

```rust
#[post("/api/v1/create_tag", crate::StateExtractor { db, session_user, .. }: crate::StateExtractor)]
pub async fn create_tag(label: String) -> Result<Tag> {
    db.create("tag")
        .content(CreateTag { owner: session_user.record_id(), label, created_at: Datetime::default() })
        .await?
        .ok_or_else(|| dioxus::CapturedError::msg("Could not save the tag."))
}
```

- **The owner comes from `session_user.record_id()`.** Never from an argument.
  The client can send anything.
- **Reads filter by owner too:** `SELECT * FROM $tag WHERE owner = $user`. The
  record id is usually in the URL, so a read without the filter lets anyone
  read anyone's row. Answer `None` for "not yours" so the response does not
  confirm the id exists.
- **Bind values, never format them into the query.**

---

## Adding a sign-in method

The session machinery stays as it is. What changes is how an account is found.

1. **A field to find the account by**, with a unique index, in
   `database/schema/user.surql`:

   ```surql
   DEFINE FIELD IF NOT EXISTS apple_id ON user TYPE option<string>;
   DEFINE INDEX IF NOT EXISTS user_apple_id_idx ON user FIELDS apple_id UNIQUE;
   ```

2. **A one-time server challenge** issued before the provider UI opens. Store a
   random state and expected nonce in the visitor's server session for no more
   than ten minutes, return the raw values to the client, and consume the
   record on the callback. On iOS, pass both to
   `start_apple_auth(&state, &nonce)`; the native plugin hashes the nonce before
   placing it on Apple's request.

3. **A callback** that verifies the credential, finds or creates the `user`
   row, and signs in:

   ```rust
   #[post("/api/v1/apple_signin_callback", crate::StateExtractor { db, auth_session, .. }: crate::StateExtractor)]
   pub async fn sign_in_apple(credential: AppleCredential) -> Result<User> {
       let challenge = consume_apple_challenge(&auth_session)?;
       ensure!(credential.state == challenge.state, "state mismatch");
       let token = exchange_apple_authorization_code(
           &credential.authorization_code,
           APPLE_CLIENT_ID,
       ).await?;
       let apple_id = verify_apple_identity_token(
           &token,
           APPLE_CLIENT_ID,
           &challenge.expected_nonce,
       ).await?;
       let user = find_or_create_user_by_apple_id(&db, apple_id).await?;
       auth_session.login_user(user.id.key.to_sql());
       auth_session.remember_user(true);
       Ok(user)
   }
   ```

   Check Apple's RS256 signature against the matching `kid`, issuer, audience,
   expiration, and nonce. Exchange the single-use authorization code at
   Apple's token endpoint using an ES256 client-secret JWT made from
   `APPLE_TEAM_ID`, `APPLE_KEY_ID`, and a server-only `.p8` private key. Use the
   stable `sub` claim as the account key. A token the client says is valid is
   still just a string.

4. **Its paths in `is_unsecured_path`.** A sign-in endpoint that requires a
   session cannot be reached.

5. **A button on `SignIn`.** On a phone, the credential comes from the
   `g3-native-plugins` `auth` feature — Sign in with Apple on iOS, Google
   Sign-In on Android. See [native-plugins.md](native-plugins.md). On success,
   call `app_state.apply_user(user)` and `animated_navigate` to the app, the
   way the guest button does.

For web, use Sign in with Apple JS in popup mode, pass the same server-issued
state and nonce, and post the returned `code`, `id_token`, `state`, and
first-login `user` object to the callback. Register every exact HTTPS return
URL against the Services ID. A popup lets the app perform the final POST from
its own origin, so a normal SameSite session cookie still binds the callback to
the challenge that started it.

### Google sign-in on the web

Let Google render the web button with the Google Identity Services JavaScript
API. Do not put a custom button in front of `google.accounts.id.prompt()`:
`prompt()` is the One Tap API, not a custom-button sign-in action. Do not mix
the HTML `g_id_onload` configuration with JavaScript `initialize()` or
`renderButton()` calls either.

Render an empty, full-width container in the sign-in component. After the
Google script loads, initialize redirect mode once and render Google's button
into that container:

```javascript
const container = document.getElementById("google-signin-button");

google.accounts.id.initialize({
  client_id: container.dataset.clientId,
  ux_mode: "redirect",
  login_uri: new URL(
    "/api/v1/google_signin_callback",
    window.location.origin,
  ).href,
});
google.accounts.id.renderButton(container, {
  type: "standard",
  theme: "outline",
  size: "large",
  text: "continue_with",
  shape: "rectangular",
  logo_alignment: "left",
  width: Math.floor(container.getBoundingClientRect().width),
});
```

The absolute `login_uri` is required by Google in deployed environments. Using
`window.location.origin` keeps a single web bundle valid on localhost, staging,
and production. The outline, large, rectangular, full-container configuration
is the closest supported match to a full-width neutral g3-ui button; Google's
iframe still owns its internal typography, spacing, and branding.

### OAuth configuration is build-time and runtime

Google's web client ID and Apple's Services ID are public identifiers, not
secrets, but a fullstack web app consumes each value in two different builds:

- the WASM client uses `option_env!` and needs the value in the environment that
  runs `dx build` or `dx bundle`;
- the server uses `std::env::var` and needs the matching value in its container
  environment when it validates the token audience.

Passing a provider ID through Compose configures only the second half. For a
container build, declare and forward the public values before bundling:

```dockerfile
ARG GOOGLE_OAUTH_CLIENT_ID
ARG APPLE_SERVICES_ID
ENV GOOGLE_OAUTH_CLIENT_ID=$GOOGLE_OAUTH_CLIENT_ID
ENV APPLE_SERVICES_ID=$APPLE_SERVICES_ID
RUN dx bundle --platform web -r
```

Then pass them as build arguments from the release workflow and as runtime
environment variables from Compose. Keep each build-time value identical to
its runtime value; otherwise the browser requests a token for an audience the
server rejects. To reuse one image across staging, homelab, and production,
configure one provider identifier with every deployed origin or return URL and
use it in every environment. Do not pass private OAuth client secrets into a
browser build.

Apple's Team ID, Key ID, and `.p8` private key are runtime server secrets. Use
`APPLE_PRIVATE_KEY` or a mounted `APPLE_PRIVATE_KEY_PATH`; never forward either
through Docker build arguments. The native audience is the bundle identifier,
while the web audience and token-exchange client ID are the Services ID.

**Upgrading a guest.** Because a guest is a real row, "attach an email to this
guest account" is an `UPDATE $user SET email = ..` on the signed-in user rather
than a second account and a data migration.

**Password sign-in** needs a slow hash (`argon2`), rate limiting on the
endpoint, and a reset flow. If you do not need passwords, prefer an OAuth
provider or an emailed one-time code.

---

## Sign-out and account deletion

`sign_out` ends the session. `delete_account` (`src/auth/account.rs`) deletes
every row the account owns and then the account, in one transaction, and ends
the session. Apple and Google both require in-app deletion for apps with
accounts.

**When you add a table with an `owner` field, add it to
`DELETE_ACCOUNT_QUERY`.** A test reads `database/schema/` and fails the build if
you forget.

## Rate limiting

The template does not ship a rate limiter. Before exposing sign-in endpoints to
the internet, put one in front of them — at a reverse proxy, or in the router
with `tower_governor`. Key it on the session cookie rather than the client IP:
Dioxus's server does not attach connection info to requests, so IP-based key
extractors never find a key.
