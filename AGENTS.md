# AGENTS.md

Instructions for coding agents working in this repository (Claude Code, Codex,
Cursor, Copilot, and anything else that reads `AGENTS.md`). Useful for people
too. Follow these over your defaults.

## What this is

A cross-platform app on the **g3 stack**: one Rust codebase for web (WASM),
Android and iOS. The notes feature is the worked example; replace it with
your own domain.

| Piece | Version | Reference |
| --- | --- | --- |
| Dioxus (fullstack, router) | 0.7.9, pinned `=` | [docs/dioxus/](docs/dioxus/README.md), [dioxuslabs.com/learn/0.7](https://dioxuslabs.com/learn/0.7/) |
| g3-ui (components, theme) | 0.4 | [docs/g3-ui.md](docs/g3-ui.md) |
| g3-route-transitions (navigation) | 0.4 | [docs/navigation.md](docs/navigation.md) |
| g3-native-plugins (device APIs) | 0.4 | [docs/native-plugins.md](docs/native-plugins.md) |
| g3-auth (session guard, `#[public]`) | 0.1 | [docs/authentication.md](docs/authentication.md) |
| g3-cache (client, server and CDN caches) | 0.2 | [docs/architecture.md](docs/architecture.md#data-and-the-caches) |
| SurrealDB + SurrealKit | 3.2.4 / 0.6.3 | [database/README.md](database/README.md) |
| Tailwind (layout utilities only) | v4, run by `dx` | [docs/styling.md](docs/styling.md) |

**Your training data is probably wrong about these.** Dioxus 0.7 changed server
functions, assets and signals from earlier versions, and the g3 crates are new.
Read the linked doc before writing code against a library, not after it fails
to compile. Prefer the patterns already in `src/` over what you remember.

## Map

```
src/app.rs              Route enum + transitions + theme. Read first.
src/state.rs            AppState: user, appearance, `changed`
src/data_change.rs      What each kind of mutation makes stale in the client cache
src/main.rs             Server: DB connection, session layers, router
src/server_url.rs       Mobile builds' server origin
src/health.rs           Container probes, merged outside the session layers
src/auth/               Session guard, StateExtractor, guest sign-in, account deletion
src/db/                 One file per table: row types + server functions
src/components/         One directory per feature, plus shell/ and shared/
database/schema/        One .surql file per table (desired state, not migrations)
docs/                   Human and agent documentation; docs/dioxus/ for the framework
tests/ui/               Playwright smoke tests
```

`src/db/note.rs` + `src/components/notes/` + `database/schema/note.surql` are
the worked example of a feature end to end: a table, cached reads, an
optimistic mutation, and the `DataChange` it reports. Copy their shape.

## Commands

```bash
just check        # web + server + mobile type-check. `cargo check` alone is NOT enough
just test         # Rust tests, web + server
just lint         # clippy (all three) + biome
just lint-strict  # the same with warnings as errors, as CI runs it
just format       # rustfmt + biome
just db-up        # SurrealDB in Docker
just dev          # dev server, http://localhost:8080
just test-ui      # Playwright
```

## Definition of done

Before you say a change is finished:

1. `just check` passes, on all three feature sets.
2. `just test` passes.
3. `just lint-strict` is clean.
4. For UI changes, you looked at it in a browser at **390×844**, and at
   **1440×900** if layout changed, and read the `dx serve` output for server
   errors. See "Verifying in a browser" below.

If you could not do one of these, say which and why.

---

## Rules

### UI: g3-ui first

- **Build screens from g3-ui components.** A page is `Header` + `Content`, a
  block is a `Card`, a row is an `Item` in a `List`, a vertical flow is a
  `Stack`, copy is `Text { variant, tone }`, an empty or failed state is an
  `EmptyState` (see `LoadFailed` in `components/shared/resource_states.rs`).
  The catalog is [docs/g3-ui.md](docs/g3-ui.md). Check it before writing any
  `div`.
- **Do not add CSS classes or rules** for things a component or prop already
  does. Tailwind is for layout only. A new rule in `tailwind.css` needs a
  reason it cannot be a component, prop or utility.
- **No color literals** outside `app_theme` in `src/app.rs`. Use
  `var(--g3-color-*)` tokens or `color-mix()` over them.
- **No viewport media queries.** Use `@container g3-app-shell (width >= 48rem)`.
- **Never override g3-ui's own class names.** If a component cannot do what is
  needed, say so rather than patching around it.
- Stateful g3-ui components take an owned `Signal` and write it:
  `Input { value: title }`, `Toggle { checked: dark }`. A control that follows
  the route rather than owning its value sets `defer_selection` and reports
  picks through `onchange` (see `NotesToolbar`).
- Feedback goes through `use_toast()`, questions through `use_alert()`.
  `AppWrapper` hosts both; do not mount another `Toast`.
- Icon-only buttons need `aria_label`.

### Components, props and hooks

The full guide, with examples, is [docs/dioxus/patterns.md](docs/dioxus/patterns.md).

- **Hooks run unconditionally, in the same order, every render.** All `use_*`
  calls at the top of the component, before any early `return`; branch on
  their results afterward. `use_effect` keeps its first closure, so an effect
  below a return that runs later still sees the values of the first render.
- **Never hold a signal borrow across `.await`.** Copy the value into a local
  first. `clippy.toml` lints for it.
- **A prop read inside a hook is a `ReadSignal<T>`.** The parent still passes a
  plain value (`NoteDetail { id }`); the child's `use_cached`, `use_memo` or
  `use_effect` then reruns when it changes. Do not use `use_reactive!`, and do
  not mirror a prop into a local signal by hand.
- **Derived values are `use_memo`**, not recomputed in the body, when they are
  read by more than one place or are costly.
- **A struct edited field by field is a `Store`** (`#[derive(Store)]`), so a
  write to one field reruns only what reads that field. Use a signal when the
  value is replaced whole.
- **Split components along what they read.** Fast-changing state (a draft, a
  drag) gets its own component, as `NoteForm` does; a read used by one
  section lives in that section; a read shared by several is a memo in the
  parent passed down as a `ReadSignal`.
- **No whole-record or whole-list clones per render.** Borrow from read
  guards in the body; rows capture an index and look up with `peek_row` when
  tapped; `sig.read().len()`, never `sig().len()`. Clone on a data change (in
  a memo) or on a tap, not on a render.
- **Never write a signal in the body.** It is a side effect: it goes in an
  event handler or a `use_effect`. The one exception, guarded and commented,
  is `AppStateProvider` applying the server's answer before SSR.
- A context's signals are created inside the `use_context_provider`
  initializer, so they belong to the provider's scope.
- A handler a busy page passes to a child as a prop is a `use_callback`, not
  a closure rebuilt on every render (see
  [docs/troubleshooting.md](docs/troubleshooting.md)).
- `AppState` is `Copy`; capture it into closures directly. A closure that
  captures only `Copy` handles is itself `Copy`.
- Format with `cargo fmt`. Do not run `dx fmt` 0.7.9: it corrupts closures
  inside `rsx!` (see [docs/dioxus/patterns.md](docs/dioxus/patterns.md#tooling)).

### Reading and changing data

- **Screens read through `use_cached(server_fn, (args,))`.** It shows the last
  answer at once (from IndexedDB, or redb on mobile) and refetches in the
  background. `use_resource` is only for reads that change per keystroke
  (search) or must not be trusted from a cache (the splash's sign-in check);
  the per-keystroke ones read `(app_state.data_version)()` to pick up
  mutations.
- **After a mutation, `app_state.changed(DataChange::..)`**, naming what
  changed. `src/data_change.rs` lists the cached reads each change makes
  stale. A new cached read that a mutation can affect goes into those lists in
  the same change.
- **Mutations say "set to", not "toggle"** (`set_note_pinned(id, true)`), so a
  device showing stale state cannot undo another device's change.
- For instant feedback, `update_cached` the reads, then call the server, then
  `changed(..)` to reconcile.
- **Handle every state:** `None` is a `Spinner` or skeleton, `Some(Err(_))` is
  `LoadFailed`, empty is an `EmptyState` saying what will appear, then the
  content.
- Show server errors through `components::shared::error_message`.

### Routes and navigation

All animation is declared on `Route` in `src/app.rs`. **No component contains
animation code.** Full vocabulary: [docs/navigation.md](docs/navigation.md).

- Layers: `layer = stack_root` (a tab, inside the first `#[layout(AppShell)]`),
  `layer = stack_page` (a page above a tab, inside the second
  `#[layout(AppShell)]`, rendering its own `Header`), `layer = sheet` (inside
  `#[layout(SheetShell)]`).
- `history = replace` for filters or tabs in the URL. `handoff_from = (A, B)`
  so Back skips screens that forward immediately. `forward_to = ..` between
  two pushed pages, so the push runs the right way; never needed to reach a
  sheet.
- **Navigate with `animated_navigate(route)`, go back with the app's
  `BackButton`** (`components/shell/back_button.rs`), which calls
  `animated_back_or_navigate`. Not `navigator.push`/`go_back`: those change
  the route before the transition snapshot. `navigator.replace` only when the
  current entry must not be returnable to (after a delete).
- A new pushed page or sheet gets a fallback in `BackButton`'s `match`.
- Add a test to `transition_tests` in `src/app.rs` for every route you add.
- **Per-screen state (filters, tabs, search) goes in the URL** as a route
  field. `AppState` is only for what spans screens.
- Never parse a path string back into a `Route`. Build the variant.

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
- Endpoints and pages are guarded by default. Marking one `#[g3_auth::public]`
  (server function) or `#[public]` (a `Route` variant) is a security decision:
  only for what must work signed out, with a comment saying why.
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
- `RecordId` has no `Display`. `record_key(&id)` for a URL segment.
- Enums stored in constrained columns need `#[surreal(untagged)]` and
  `#[surreal(value = "..")]` per variant, plus the storage test from
  `src/db/user.rs`.

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

- Comment **why**, not what: a decision, a constraint, a failure mode not
  visible in the code. The existing comments set the tone and density.
- Prefer a Rust unit test over a browser test. Browser tests are for
  transitions, gestures, the responsive layout and hydration.
- In Playwright, query by role and accessible name, never by CSS class.
- Do not add dependencies without a reason tied to the task. Pin `dioxus*` and
  `manganis` exactly (`=0.7.9`); they must match the `dx` CLI.
- Commits follow Conventional Commits (`feat:`, `fix:`, `chore:`).

---

## Verifying in a browser

```bash
just db-up
just dev                  # or reuse a server already on :8080
```

- Check `http://127.0.0.1:8080` first; reuse a running server rather than
  starting a second one. If you start one, stop only that one afterward.
- Non-interactive start: `dx serve --web --addr 127.0.0.1 --port 8080 --open false --interactive false`.
- Set the viewport to **390×844** before interacting; this is a mobile-first
  app. Then check **1440×900** for the desktop rail if layout changed.
- Sign in with **Continue as guest**. It creates a fresh account each time.
- A sheet or push transition takes ~400ms. Wait before screenshotting, or you
  capture the page mid-animation.
- Stop `dx serve` before editing Rust: a failed hot-patch can leave a build
  that never loads (see [docs/troubleshooting.md](docs/troubleshooting.md)).
- Read the `dx serve` output for `[500]`s and panics, and the browser console
  for `Error deserializing data` (a hydration mismatch).

## Where to look when stuck

- [docs/troubleshooting.md](docs/troubleshooting.md): known failure modes and fixes
- [docs/dioxus/patterns.md](docs/dioxus/patterns.md): Dioxus as used here, and its sharp edges
- [docs/architecture.md](docs/architecture.md): how a request flows, and the caches
- [docs/adding-a-feature.md](docs/adding-a-feature.md): table → server functions → route → screen
- The g3 crates' READMEs and source, when a doc here is not enough:
  [g3-ui](https://github.com/g3techhq/g3-ui),
  [g3-route-transitions](https://github.com/g3techhq/g3-route-transitions),
  [g3-native-plugins](https://github.com/g3techhq/g3-native-plugins),
  [g3-auth](https://github.com/g3techhq/g3-auth),
  [g3-cache](https://github.com/g3techhq/g3-cache)
