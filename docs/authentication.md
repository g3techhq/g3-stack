# Authentication

Sessions, the auth guard, sign-in, and account deletion. The code is in
`src/auth/`.

---

## What ships

- **Guest sign-in** (`src/auth/guest.rs`). One tap creates a real `user` row and
  a real session. Every screen past sign-in can assume a signed-in user; there
  is no half-signed-in state to design around.
- **Session cookies** stored in SurrealDB (`axum_session`, with the store from
  [`g3-auth`](https://crates.io/crates/g3-auth)), resolved to a `SessionUser`
  on each request by `axum_session_auth`.
- **A guard** that rejects unauthenticated requests by default.
- **Sign-out** and **account deletion**, both from Settings.

## The pieces, in request order

```
request
  → SessionLayer         reads the cookie, loads the session row
  → AuthSessionLayer     resolves it to a SessionUser
  → require_session      401 or redirect, unless the path is public
  → your server function StateExtractor { db, session_user, auth_session }
```

`src/auth/session.rs` names these for the app: `AppUser` describes the
account table, and `StateExtractor` is `g3_auth::SessionContext` for it.

### Guarded by default

The guard (`g3_auth::require_session`, installed in `main.rs` with
`AuthGuard::for_routes(Route::Splash {})`) rejects every request without a
signed-in user, except for:

- static assets and `/.well-known/` files (deep-link association files),
- pages marked `#[public]` on `Route` in `app.rs`, and
- server functions marked `#[g3_auth::public]`.

A server function or page you add is protected without you doing anything.
Opening one up is one line beside it, with a comment saying why:

```rust
/// Public: the splash asks this before it knows whether anyone is signed in.
#[g3_auth::public]
#[get("/api/v1/is_signed_in", crate::StateExtractor { auth_session, .. }: crate::StateExtractor)]
pub async fn is_signed_in() -> Result<bool> { .. }
```

Tests in `src/auth/session.rs` pin both lists, so opening anything up always
shows as a reviewed change. Mark something public only when it genuinely has
to answer a visitor with no session: a sign-in endpoint, an OAuth callback, a
public share link, a legal page a store listing links to.

Pages are matched against their own `#[route]` and `#[nest]` patterns, never
by parsing a path into `Route`: the catch-all `#[redirect]` would turn every
unknown path, `/api/` ones included, into the splash, which is public.

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
Google and Apple are `g3-auth` features, so the button, the callback, the token
checks and the CSRF protection are not yours to write:

```toml
g3-auth = { version = "0.1", features = ["google", "apple"] }
```

1. **A way to find the account**, in `database/schema/user.surql`, with a unique
   index (`google_id`, `apple_id`, or an `identity` table keyed on provider and
   subject if you want several). Key on the provider's stable `subject`, never
   on the email.

2. **Load the schemas.** Copy `g3_auth::APPLE_SCHEMA` into
   `database/schema/apple_challenge.surql` beside `sessions.surql`: each Apple
   attempt starts with a single-use `state` and `nonce` kept there for ten
   minutes.

3. **Say how an identity becomes a `user`** by implementing `GoogleAccounts`
   and `AppleAccounts` (one `find_or_create` each), and merge the routers
   **before** the session layers in `main.rs`:

   ```rust
   let router = dioxus::server::router(app::App)
       .merge(g3_auth::google::google_router::<auth::AppUser, Client, _>(
           GoogleConfig::new("/", "/welcome"),
           auth::Accounts,
       ))
       .merge(g3_auth::apple::apple_router::<auth::AppUser, Client, _>(
           AppleConfig::new("/", "/welcome").native_client_id("com.example.app"),
           auth::Accounts,
       ))
       .layer(/* extension, guard, auth session, session store */);
   ```

   Link to an existing account by email only when the provider says the email
   is verified (`email_verified`); an unverified one must never take over an
   account.

4. **Buttons on `SignIn`:** `GoogleSignIn {}` and `AppleSignIn {}`. On the web
   they start the provider's redirect flow and the server redirects the browser
   when it is done; on a phone they use the system sheet (Google on Android,
   Apple on iOS) and call `on_signed_in`, where you call
   `app_state.apply_user(user)` and `animated_navigate` the way the guest button
   does. iOS also needs the Sign in with Apple capability, and an app that
   offers Google on iOS must offer Apple.

5. **Delete means revoke.** With an Apple signing key configured, `AppleIdentity`
   carries a refresh token; keep it with the account and call
   `g3_auth::apple::revoke_token(client_id, refresh_token)` from
   `delete_account`, or App Store review (guideline 5.1.1(v)) can reject the
   app.

### What `g3-auth` checks for you

- **Login CSRF.** Both providers post a signed token from their own page, where
  the `SameSite=Lax` session cookie is not sent, so nothing but the provider's
  cookie ties the post to the browser that started it. A browser post (it
  carries `Origin`) must have a cookie equal to its form field: `g_csrf_token`
  for Google, an `apple_state` cookie the Apple button sets for `state`. Without
  it a page could sign a visitor in as the attacker's own account. A native
  app's post has no `Origin` and no cookie jar, and is let through. Do the same
  with `g3_auth::csrf::protect` for any callback you add.
- **Apple's token.** Signature against Apple's keys, issuer, audience, expiry and
  the attempt's nonce; the challenge is consumed once. With
  `APPLE_TEAM_ID`, `APPLE_KEY_ID` and `APPLE_PRIVATE_KEY` the authorization code
  is exchanged with Apple too.
- **A token the client says is valid is still just a string**, so never
  trust one you did not verify.

### One JWT backend

If your own code uses `jsonwebtoken` (an App Store or Play check, say), enable
its **`aws_lc_rs`** feature, not `rust_crypto`. SurrealDB already turns on
`aws_lc_rs` in the same dependency graph, and with both on `jsonwebtoken` panics
on first use ("Could not automatically determine the process-level
CryptoProvider"). A debug build compiles fine and the first request that signs
or verifies a token fails.

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
