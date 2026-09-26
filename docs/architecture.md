# Architecture

How the pieces fit, and why they are arranged this way. To add code, see
[adding-a-feature.md](adding-a-feature.md).

---

## One codebase, three builds

`src/` compiles for every target. Cargo features decide what each build
contains:

| Feature | Target | Contains |
| --- | --- | --- |
| `web` (default) | `wasm32-unknown-unknown` | The client, as WASM |
| `mobile` | Android / iOS | The client, in a native WebView |
| `server` | Host | axum, the SurrealDB client, and the bodies of every server function |

The `#[get]`/`#[post]` macros are what make that work. On a server build they
expand to an axum handler; on a client build the body is discarded and replaced
with a typed HTTP call. The same function is one thing to write and one thing to
call:

```rust
// Definition (src/db/note.rs) — the body exists only in the server build.
#[post("/api/v1/create_note", crate::StateExtractor { db, session_user, .. }: crate::StateExtractor)]
pub async fn create_note(title: String, body: String) -> Result<Note> { /* .. */ }

// Call site (src/components/notes/note_editor.rs) — ordinary async Rust.
create_note(title, body).await
```

Types cannot drift between client and server: there is one definition, and a
mismatch is a compile error rather than a runtime surprise.

**Check all three.** A bare `cargo check` builds only the web feature set, so a
server-only or mobile-only break stays invisible until CI. `just check` does all
three.

---

## A request, end to end

```
  Browser / WebView
        │
        │  POST /api/v1/create_note   { "title": "...", "body": "..." }
        ▼
  ┌─────────────────────────────────────────────────────┐
  │ SessionLayer          reads the cookie, loads the   │
  │                       session row from SurrealDB    │
  ├─────────────────────────────────────────────────────┤
  │ AuthSessionLayer      resolves it to a SessionUser  │
  ├─────────────────────────────────────────────────────┤
  │ auth_check            401 (or redirect) if the path │
  │                       is not allowlisted and there  │
  │                       is no session                 │
  ├─────────────────────────────────────────────────────┤
  │ Extension(AppServerState)   the shared DB handle    │
  └─────────────────────────────────────────────────────┘
        │
        ▼
  StateExtractor  { db, auth_session, session_user }
        │
        ▼
  create_note()  ──►  SurrealDB
```

Two ordering facts in `src/main.rs` are load bearing:

**Layers apply bottom-up.** `SessionLayer` is listed last and runs first,
because `AuthSessionLayer` needs a session before it can resolve a user, and
`auth_check` needs a resolved user before it can decide anything.

**The health router is merged, not layered, and merged last.** `Router::layer`
only wraps routes registered before it, so merging afterwards keeps probes out
of the session and auth path. A probe every 30 seconds would otherwise allocate
a session row each time. A test in `src/health.rs` asserts the ordering, because
nothing else would notice if it changed.

### StateExtractor

Every server function extracts the same three things, so they are bundled.
Destructure what you use and let `..` take the rest:

```rust
#[post("/api/v1/create_note", crate::StateExtractor { db, session_user, .. }: crate::StateExtractor)]
```

Its `Rejection` type is deliberately not `()`. axum renders `()` as an empty
200, which the client decodes as `null` — so a missing `AuthSessionLayer` would
reach `get_current_user` as an ordinary `Ok(None)` ("nobody is signed in") and
quietly bounce a signed-in user to sign-in. A misconfigured layer stack has to
look like the server fault it is.

### Where the client sends calls

A web client calls the origin that served it. A mobile client has no origin, so
`src/server_url.rs` compiles one in from `SERVER_URL` — validated as an HTTPS
bare origin in release builds — and `dioxus-cookie` gives it a cookie jar so the
session survives between calls. See [mobile.md](mobile.md).

---

## Auth

Covered in full in [authentication.md](authentication.md). The shape:

- **Guarded by default.** `auth_check` rejects any path not on
  `is_unsecured_path`'s allowlist.
- **401 for a fetch, redirect for a navigation**, decided by `Sec-Fetch-Dest`
  (or the `/api/` prefix without it). A redirect served to a `fetch` comes back
  as HTML the caller cannot decode.
- **One place decides.** The splash at `/` asks the server once and routes
  onward. It is `/` because that is where both paths arrive: a signed-out web
  page load by redirect, a native launch by default.

---

## State

**Per-screen state goes in the URL. Cross-cutting state goes in `AppState`.**

A filter, a tab, a search term belongs in the route: it survives a refresh,
Back walks through it, and a filtered view is something you can link to.

`AppState` (`src/state.rs`) is for what genuinely spans screens — the signed-in
user, the appearance, the shared toast and sign-out confirmation. Every field is
a `Signal`, which is `Copy`, so `AppState` itself is `Copy` and captures into
`move` closures and `spawn(async move { .. })` without a borrow-checker
argument. That is why it is a struct of signals rather than a signal of a
struct.

`data_version` is the refetch mechanism: a mutation bumps it, and any
`use_resource` that reads it refetches. Deliberately blunt — it refetches more
than strictly necessary, and the alternative is a per-entity cache scheme that
is a lot of machinery to get subtly wrong at this size.

### The first paint is already themed

`AppStateProvider` loads the signed-in user with `use_server_future`, not
`use_resource`. The server waits for the answer and renders the page in the
user's saved mode and color scheme, and the client hydrates that data rather
than refetching. A returning user never sees the default theme flash and swap.

Two consequences, both handled in the code and worth knowing:

- The provider applies the user with `peek`, not a tracked read, because it
  also writes those signals. A tracked read there is a render loop — and during
  server rendering, which runs until nothing is dirty, a request that never
  returns.
- Head elements (`document::Link`) live *inside* the provider, in
  `ThemedShell`. Fullstack hydrates head elements and server futures from one
  ordered stream. The provider suspends on the server, so anything beside it
  would be written to that stream before its children on the server but after
  them on the client, and the client would decode one entry as another.

---

## Rendering and transitions

```
App
└── AppStateProvider          AppState, loads the user once (server future)
    └── ThemedShell           head links, then:
        └── AppWrapper        theme tokens as CSS custom properties, iOS/MD mode
            └── NativePluginsProvider
                └── Router
                    └── RootLayout          native back navigation
                        ├── Splash, SignIn
                        ├── AppShell        tabs: header, Outlet, tab bar
                        ├── NewNote, EditNote            (cover)
                        ├── PushedPageLayout
                        │   └── NoteDetail               (pushed)
                        └── AppOverlays     toast, sign-out confirmation
```

`AppWrapper` writes the theme as inline CSS custom properties on the shell
element, so a new `Theme` re-themes the whole tree without remounting it —
navigation, scroll position, and half-typed form state survive a light/dark
switch.

`AppOverlays` sits in `RootLayout`: inside the router (signing out navigates,
and `use_navigator` panics outside one) but above every route, so a backdrop
covers the whole frame and a toast raised just before a navigation outlives it.

`AppShell` is the responsive part. The same tree renders as a bottom tab bar on
a phone and a left rail from 48rem — a container query on the shell's width, not
the viewport's, so an app embedded in a wide page keeps its phone layout.

Transitions are declared on the route enum and covered in
[navigation.md](navigation.md).

### Mode and platform move together

g3-ui's `mode` (which look to draw) and g3-route-transitions' `Platform` (which
motion to animate with) are separate thread-local globals that components read
while rendering. They must agree, or a Material screen animates with iOS
motion. `state::apply_mode` sets both; nothing should set one alone.

### Server rendering and hydration

The server renders HTML that the WASM client hydrates. **The rendered tree must
be identical in both builds.** Markup gated on `cfg` breaks hydration in ways
that surface as unrelated event handlers doing nothing. Gate the *call*, not the
markup — see [native-plugins.md](native-plugins.md) for the pattern.

`NativePluginsProvider` is mounted unconditionally for the same reason: it
renders only its children, and installs its context only on targets that have
one.

---

## Database

`database/schema/` is the *desired* state, one file per table, not a migration
log. SurrealKit diffs it against the running database, which is why every
statement is `IF NOT EXISTS`.

Debug builds apply it at startup, so editing a `.surql` file and restarting
`dx serve` is the whole local workflow. Release builds deliberately do not: a
production schema change is a reviewed rollout, not a side effect of a deploy.
See [../database/README.md](../database/README.md).

Every table is `PERMISSIONS NONE`. Nothing connects to SurrealDB but the server,
which authenticates as root; authorization is the `owner = $user` clause in
each query. Row-level database permissions would be a second source of truth
about the same question.
