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
- Read `docs/dioxus/patterns.md` if you have not this session.
- Decide the route layer now (`stack_root` tab, `stack_page` page, `sheet`)
  using `docs/navigation.md`. Say which and why in one line.

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

## 3. Cache invalidation — `src/data_change.rs`

- Add a `DataChange` variant for the new mutations, listing every cached read
  whose query touches the tables they write (including other features' reads).

## 4. Route — `src/app.rs`

- Add the variant with `#[transition(layer = ..)]` in the right layout: the
  first `AppShell` block for tabs, `SheetShell` for sheets, the last
  `AppShell` block for pushed pages.
- Per-screen state (filters, tabs) as route fields, with `history = replace`.
- `forward_to` on pages that open another pushed page.
- Add assertions to `transition_tests` if the motion is not obvious.
- Add a fallback for it in `src/components/shell/back_button.rs`.

## 5. Screen — `src/components/<feature>/`

- Pushed pages and sheets render `Header { title, start: rsx! { BackButton {} } }`
  then `Content { .. }`. Tabs render inside `AppShell` and add their title to
  its `match`.
- All hooks first, before any early return. Reads are
  `use_cached(server_fn, (args,))`; route and component props read inside a
  hook are `ReadSignal<T>`. No `use_reactive!`, no signal writes in the body.
- Render four states: `Spinner`; `LoadFailed`; an `EmptyState` saying what
  will appear; the content (usually `List`/`Item`), borrowed from the read
  guard, rows capturing an index (`peek_row`) rather than cloning ids.
- A form's draft lives in its own component, seeded from props (`NoteForm`).
- Mutations: call the server function, then
  `app_state.changed(DataChange::..)`, with `use_toast()` for feedback and
  `error_message` for server errors. For instant feedback, `update_cached`
  first. Mutations take the target state, not a toggle.
- Navigate with `animated_navigate`; back is `BackButton`.
- g3-ui components only. No new CSS classes; copy is `Text { tone }`;
  Tailwind only for layout.
- Register with `mod`/`pub use` in the feature's `mod.rs` and in
  `src/components/mod.rs`.

## 6. Finish

```bash
just check && just test && just lint-strict
```

Then use the `verify-in-browser` skill to walk the new flow at 390×844. Report
what you checked, and anything you could not.
