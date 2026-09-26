---
name: add-feature
description: Add a feature to this g3 stack app end to end — SurrealDB table, Rust row types, server functions, route with transition, and a g3-ui screen. Use when asked to add a new entity, resource, CRUD screen, or page backed by data.
---

# Add a feature

Build it in dependency order. Each step compiles before the next starts. The
worked example to copy is notes: `database/schema/note.surql`, `src/db/note.rs`,
`src/components/notes/`. The long-form guide is `docs/adding-a-feature.md`.

## 0. Before writing anything

- Read `AGENTS.md` if you have not this session.
- Read `docs/g3-ui.md` for the components you will need. Do not guess props.
- Decide the route layer now (`root` tab, `pushed` page, `cover` sheet) using
  `docs/navigation.md`. Say which and why in one line.

## 1. Schema — `database/schema/<table>.surql`

- `DEFINE TABLE IF NOT EXISTS <t> TYPE NORMAL SCHEMAFULL PERMISSIONS NONE;`
- `owner` as `TYPE record<user>` if rows belong to a user, plus
  `DEFINE INDEX IF NOT EXISTS <t>_owner_idx ON <t> FIELDS owner;`
- Index every other field a query filters by.
- If it has an `owner`: add `DELETE <t> WHERE owner = $user;` to
  `DELETE_ACCOUNT_QUERY` in `src/auth/account.rs`, before `DELETE $user`.

## 2. Types and server functions — `src/db/<table>.rs`

- Row struct: `#[derive(Serialize, Deserialize, SurrealValue, Clone, PartialEq, Debug)]`, `impl_id!(..)`.
- Insert struct without `id`, with `#[cfg_attr(not(feature = "server"), allow(dead_code))]`.
- Validation as a plain `fn validate_..(..) -> Result<.., String>` with unit tests.
- Server functions with `crate::StateExtractor { db, session_user, .. }`:
  - owner from `session_user.record_id()`, never an argument;
  - every read and write filtered by `WHERE owner = $user`;
  - values bound with `.bind`, never formatted;
  - `#[get]` reads, `#[post]` writes, paths under `/api/v1/`.
- Register in `src/db/mod.rs`.
- Run `just check`.

## 3. Route — `src/app.rs`

- Add the variant with `#[transition(..)]` in the right layout
  (`AppShell` for tabs, `PushedPageLayout` for pushed pages, between them for
  covers).
- Per-screen state (filters, tabs) as route fields, with `replace`.
- Add assertions to `transition_tests` for what the route should do.
- Add a fallback for it in `src/components/shell/back_button.rs` if it has a
  back button.

## 4. Screen — `src/components/<feature>/`

- Pushed pages and covers wrap themselves in `PageShell { title: .., end_button: .. }`.
  Tabs render inside `AppShell` and add their title to its `match`.
- All hooks first. `use_resource` reading `(app_state.data_version)()`;
  `use_reactive!` for props that feed it.
- Render four states: `Spinner { center: true }`; a `Card` with the error and a
  retry `Button`; a `Card` empty state; the content (usually `List`/`Item`).
- Mutations: call the server function in `spawn`, then `app_state.bump_data()`
  and `app_state.show_toast(..)`; navigate with `animated_navigate` /
  `animated_go_back`.
- g3-ui components only. No new CSS classes; supporting text uses
  `g3-message-text g3-message-text-muted`; Tailwind only for layout.
- Register with `mod`/`pub use` in the feature's `mod.rs` and in
  `src/components/mod.rs`.

## 5. Finish

```bash
just check && just test && just lint
```

Then use the `verify-in-browser` skill to walk the new flow at 390×844. Report
what you checked, and anything you could not.
