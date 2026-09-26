# AGENTS.md

Instructions for coding agents working in this repository (Claude Code, Codex,
Cursor, Copilot, and anything else that reads `AGENTS.md`). Useful for people
too. Follow these over your defaults.

## What this is

A cross-platform app on the **g3 stack**: one Rust codebase for web (WASM),
Android, and iOS.

| Piece | Version | Reference |
| --- | --- | --- |
| Dioxus (fullstack, router) | 0.7.9 | [dioxuslabs.com/learn/0.7](https://dioxuslabs.com/learn/0.7/) |
| g3-ui — components, theme | 0.3.0 | [docs/g3-ui.md](docs/g3-ui.md) |
| g3-route-transitions — navigation | 0.3.0 | [docs/navigation.md](docs/navigation.md) |
| g3-native-plugins — device APIs | 0.4.0 | [docs/native-plugins.md](docs/native-plugins.md) |
| SurrealDB + SurrealKit | 3.2.4 / 0.6.3 | [database/README.md](database/README.md) |
| axum + axum_session | 0.8 / 0.20 | [docs/authentication.md](docs/authentication.md) |
| Tailwind (layout utilities only) | v4, run by `dx` | [docs/styling.md](docs/styling.md) |

**Your training data is probably wrong about these.** Dioxus 0.7 changed server
functions, assets, and signals from earlier versions, and the g3 crates are new.
Read the linked doc before writing code against a library, not after it fails to
compile. Prefer the patterns already in `src/` over what you remember.

## Map

```
src/app.rs              Route enum + transitions + theme. Read first.
src/state.rs            AppState: user, appearance, shared overlays
src/main.rs             Server: DB connection, session layers, router
src/server_url.rs       Mobile builds' server origin
src/auth/               Session guard, StateExtractor, sign-in, account deletion
src/db/                 One file per table: row types + server functions
src/components/         One directory per feature, plus shell/ and settings.rs
database/schema/        One .surql file per table (desired state, not migrations)
docs/                   Human and agent documentation
tests/ui/               Playwright smoke tests
```

`src/db/note.rs` + `src/components/notes/` + `database/schema/note.surql` are
the worked example of a feature end to end. Copy their shape.

## Commands

```bash
just check        # web + server + mobile type-check. `cargo check` alone is NOT enough
just test         # Rust tests, web + server
just lint         # clippy (both) + biome
just format       # rustfmt + biome
just dev          # dev server, http://localhost:8080 (needs `just db-up`)
just test-ui      # Playwright
```

## Definition of done

Before you say a change is finished:

1. `just check` passes — all three feature sets.
2. `just test` passes.
3. `just lint` is clean.
4. For UI changes, you looked at it in a browser at **390×844**, and at
   **1440×900** if layout changed, and read the `dx serve` output for server
   errors. See "Verifying in a browser" below.

If you could not do one of these, say which and why.

---

## Rules

### UI: g3-ui first

- **Build screens from g3-ui components.** A block of content is a `Card`, a row
  is an `Item`, a group is `List { inset: true }`, a primary action is a
  `Button`, empty and error states are a `Card`. The full catalog with props is
  [docs/g3-ui.md](docs/g3-ui.md) — check it before writing any `div`.
- **Do not add CSS classes or rules** for things a component or prop already
  does. Supporting copy uses `class: "g3-message-text g3-message-text-muted"`.
  Tailwind is for layout only (`mx-auto max-w-md`, `whitespace-pre-wrap`). A new
  rule in `tailwind.css` needs a reason it cannot be a component, prop, or
  utility.
- **No color literals.** Use `var(--color-*)` tokens or `color-mix()` over them.
- **No viewport media queries.** Use `@container g3-app-shell (width >= 48rem)`.
- **Never override g3-ui's own class names** (`.g3-card`, `.g3-btn-*`). If a
  component cannot do what is needed, say so rather than patching around it.
- Stateful g3-ui components take an owned `Signal` and write it: `Toggle {
  checked: dark }`. Do not add a value prop plus an `onchange` that sets it.
- Optional props need no `Some(..)` and string props take `&str`:
  `Card { title: "Notes" }`.
- Icon-only buttons need `aria_label`.

### Components and hooks

- **Hooks run unconditionally, in the same order, every render.** All `use_*`
  calls at the top of the component; branch on their results afterward. A hook
  inside `if`/`match` panics when the branch changes.
- **Never hold a signal borrow across `.await`.** Copy the value into a local
  first. `clippy.toml` lints for it.
- **Handle every state of a `use_resource`:** `None` → `Spinner { center: true }`;
  `Some(Err(e))` → a `Card` with the error and a retry `Button`; empty → a `Card`
  saying what will appear; otherwise the content.
- A prop feeding a resource needs `use_reactive!`:
  `use_resource(use_reactive!(|id| async move { get_note(id).await }))`.
- A resource showing data another screen can change reads
  `(app_state.data_version)()` inside its closure. After a mutation, call
  `app_state.bump_data()`.
- User feedback goes through `app_state.show_toast(message, StatusColor::..)`.
  Do not mount another `Toast`.
- `AppState` is `Copy`; capture it into closures directly.

### Routes and navigation

All animation is declared on `Route` in `src/app.rs`. **No component contains
animation code.** Full vocabulary: [docs/navigation.md](docs/navigation.md).

- Layers: `root` (a tab), `pushed` (a page above a tab), `cover` (a sheet), `morph`,
  `base` (default).
- `replace` for filters or tabs in the URL. `replaces = (A, B)` so Back skips
  screens that forward immediately. `push(group = .., order = ..)` for wizard
  steps.
- `forward = Route` **only** between two `pushed` routes. Pointing it at a
  `cover` makes the sheet slide sideways.
- New pushed routes go in `#[layout(PushedPageLayout)]`, tabs in
  `#[layout(AppShell)]`. A trailing `#[end_layout]` with nothing after it is a
  parse error.
- **Navigate with `animated_navigate(route)`; go back with
  `animated_go_back(fallback)`.** Not `navigator.push`/`go_back` — those change
  the route before the transition snapshot. `navigator.replace` only when the
  current entry must not be returnable to (after a delete).
- Add a test to `transition_tests` in `src/app.rs` for every route you add.
- Add a fallback to the `match` in `src/components/shell/back_button.rs` for a
  new pushed page or sheet.
- **Per-screen state (filters, tabs, search) goes in the URL**, as a route field.
  `AppState` is only for what spans screens.

### Server functions

```rust
#[post("/api/v1/create_note", crate::StateExtractor { db, session_user, .. }: crate::StateExtractor)]
pub async fn create_note(title: String, body: String) -> Result<Note> { .. }
```

- `#[get]` for reads, `#[post]` for writes. Path under `/api/v1/`.
- **Owner fields come from `session_user.record_id()`, never from arguments.**
- **Reads filter by owner:** `SELECT * FROM $note WHERE owner = $user`. Return
  `None` for "not yours".
- **Bind values** with `.bind(("name", value))`. Never `format!` into a query.
- Validation is a plain `fn` in the same file, called by both the form and the
  server function, with unit tests.
- Errors: `dioxus::CapturedError::msg("Human-readable message.")`.
- Endpoints are guarded by default. Adding to `is_unsecured_path` in
  `src/auth/session.rs` is a security decision — only for what must work signed
  out.
- Changing a signature or shared type: update every caller in the same change,
  then `just check`.

### SurrealDB

- Schema changes go in `database/schema/<table>.surql` with `IF NOT EXISTS`.
  Debug builds apply them on `dx serve` start. Never write migrations by hand.
- Every table: `SCHEMAFULL PERMISSIONS NONE`. Index every field you filter by.
- **A table with an `owner` field must be added to `DELETE_ACCOUNT_QUERY` in
  `src/auth/account.rs`.** A test enforces it.
- Row types derive `Serialize, Deserialize, SurrealValue, Clone, PartialEq,
  Debug`. The insert shape is a separate struct without `id`, marked
  `#[cfg_attr(not(feature = "server"), allow(dead_code))]`.
- `RecordId` has no `Display`. `record_key(&id)` for a URL segment;
  `ToSql::to_sql()` for SurrealQL.
- Enums stored in constrained columns need `#[surreal(untagged)]` and
  `#[surreal(value = "..")]` per variant, plus the storage test from
  `src/db/user.rs`.
- Prefer typed calls (`db.create("note").content(..)`) when the struct matches
  the schema; SurrealQL when you need a `WHERE`.

### Cross-platform

- `src/` compiles three ways. Server-only code sits behind
  `#[cfg(feature = "server")]`; server function *bodies* are already
  server-only.
- **Gate calls, never markup.** The server renders HTML the client hydrates; a
  tree that differs between builds silently kills event handlers. For native
  plugins:
  `#[cfg(any(all(feature = "web", target_arch = "wasm32"), target_os = "android", target_os = "ios"))]`
  on the `use_context` line and `cfg_if::cfg_if!` inside the handler. See
  `NativePluginDemos` in `src/components/settings.rs`.
- Head elements (`document::Link` etc.) go inside `ThemedShell`, not beside a
  component that calls `use_server_future`.
- Change appearance through `AppState::set_appearance` / `state::apply_mode`.
  Never set g3-ui's mode or g3-route-transitions' platform alone.

### Code style

- Comment **why**, not what: a decision, a constraint, a failure mode not visible
  in the code. The existing comments set the tone and density.
- Prefer a Rust unit test over a browser test. Browser tests are for transitions,
  gestures, the responsive layout, and hydration.
- In Playwright, query by role and accessible name, never by CSS class.
- Do not add dependencies without a reason tied to the task. Pin `dioxus*` and
  `manganis` exactly (`=0.7.9`); they must match the `dx` CLI.

---

## Verifying in a browser

```bash
just db-up
just dev                  # or reuse a server already on :8080
```

- Check `http://127.0.0.1:8080` first; reuse a running server rather than
  starting a second one. If you start one, stop only that one afterward.
- Non-interactive start: `dx serve --web --addr 127.0.0.1 --port 8080 --open false --interactive false`.
- Set the viewport to **390×844** before interacting — this is a mobile-first
  app. Then check **1440×900** for the desktop rail if layout changed.
- Sign in with **Continue as guest**. It creates a fresh account each time.
- Work from fresh DOM snapshots; elements are found by role and label.
- A cover or push transition takes ~400ms. Wait before screenshotting, or you
  capture the page mid-animation.
- Read the `dx serve` output for `[500]`s and panics, and the browser console
  for `Error deserializing data` (a hydration mismatch — see
  [docs/troubleshooting.md](docs/troubleshooting.md)).

## Where to look when stuck

- [docs/troubleshooting.md](docs/troubleshooting.md) — known failure modes and fixes
- [docs/architecture.md](docs/architecture.md) — how a request flows, why the layers are ordered as they are
- [docs/adding-a-feature.md](docs/adding-a-feature.md) — table → server functions → route → screen
- The g3 crates' READMEs and source, when a doc here is not enough:
  [g3-ui](https://github.com/g3techhq/g3-ui),
  [g3-route-transitions](https://github.com/g3techhq/g3-route-transitions),
  [g3-native-plugins](https://github.com/g3techhq/g3-native-plugins)
