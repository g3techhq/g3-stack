# Adding a feature

The notes feature is the reference implementation. This walks through building
the equivalent from scratch, in the order the pieces depend on each other:
table, Rust types, server functions, cache invalidation, route, screen.

The example: **tags**, so notes can be labelled.

> Using a coding agent? The `add-feature` skill in `.claude/skills/` walks it
> through these same steps, and [AGENTS.md](../AGENTS.md) holds the rules.

---

## 1. The table

One file per table under `database/schema/`. Files describe the end state —
SurrealKit diffs them against the running database rather than replaying
migrations — so `IF NOT EXISTS` on every statement is the house style.

`database/schema/tag.surql`:

```surql
DEFINE TABLE IF NOT EXISTS tag TYPE NORMAL SCHEMAFULL PERMISSIONS NONE;

DEFINE FIELD IF NOT EXISTS owner ON tag TYPE record<user>;
DEFINE FIELD IF NOT EXISTS label ON tag TYPE string;
DEFINE FIELD IF NOT EXISTS created_at ON tag TYPE datetime DEFAULT time::now();

-- Every query filters by owner first; without this each is a table scan.
DEFINE INDEX IF NOT EXISTS tag_owner_idx ON tag FIELDS owner;

-- One label per person, not one globally.
DEFINE INDEX IF NOT EXISTS tag_owner_label_idx ON tag FIELDS owner, label UNIQUE;
```

`PERMISSIONS NONE` is deliberate. Only the server connects to SurrealDB, as root;
authorization is the `owner = $user` clause in each query.

Restart `dx serve` and the schema is applied — debug builds sync on every boot.

**The table has an `owner`, so deleting an account must delete its rows.** Add
a line to `DELETE_ACCOUNT_QUERY` in `src/auth/account.rs`, before `DELETE $user`:

```surql
DELETE tag WHERE owner = $user;
```

`just test` fails until you do.

## 2. The Rust types

`src/db/tag.rs`:

```rust
use super::impl_id;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use surrealdb_types::{Datetime, RecordId, SurrealValue};

/// Mirrors `database/schema/tag.surql`.
#[derive(Serialize, Deserialize, SurrealValue, Clone, PartialEq, Debug)]
pub struct Tag {
    pub id: RecordId,
    pub owner: RecordId,
    pub label: String,
    pub created_at: Datetime,
}

impl_id!(Tag);

/// The insert shape. Separate from `Tag` rather than making `id` optional, so
/// code holding a `Tag` can always name its id without unwrapping.
#[cfg_attr(not(feature = "server"), allow(dead_code))]
#[derive(Serialize, Deserialize, SurrealValue, Clone, PartialEq, Debug)]
pub struct CreateTag {
    pub owner: RecordId,
    pub label: String,
    pub created_at: Datetime,
}
```

Each derive does a different job: `SurrealValue` moves the struct in and out of
the database, `Serialize`/`Deserialize` carry it to the client, and `PartialEq`
lets Dioxus skip a re-render when nothing changed.

Register it in `src/db/mod.rs`:

```rust
mod tag;
pub use tag::*;
```

**An enum stored in a constrained column** needs `#[surreal(untagged)]` and a
`#[surreal(value = "..")]` per variant, or it stores as `{ Variant: {} }` and
the first insert fails. Copy `AppearanceMode` in `src/db/user.rs`, including its
test.

## 3. The server functions

Same file. The bodies compile only into the server build; client builds get a
generated HTTP call with the same signature.

```rust
/// Validation as a plain function: it compiles into both builds, so the form
/// can show the message without a round trip, and the server still runs it.
pub fn validate_tag(label: &str) -> Result<String, String> {
    let label = label.trim();
    if label.is_empty() {
        return Err("A tag needs a label.".into());
    }
    Ok(label.to_string())
}

#[get("/api/v1/tags", crate::StateExtractor { db, session_user, .. }: crate::StateExtractor)]
pub async fn list_tags() -> Result<Vec<Tag>> {
    Ok(db
        .query("SELECT * FROM tag WHERE owner = $user ORDER BY label ASC")
        .bind(("user", session_user.record_id()))
        .await?
        .take::<Vec<Tag>>(0)?)
}

#[post("/api/v1/create_tag", crate::StateExtractor { db, session_user, .. }: crate::StateExtractor)]
pub async fn create_tag(label: String) -> Result<Tag> {
    let label = validate_tag(&label).map_err(dioxus::CapturedError::msg)?;

    db.create("tag")
        .content(CreateTag {
            owner: session_user.record_id(),
            label,
            created_at: Datetime::default(),
        })
        .await?
        .ok_or_else(|| dioxus::CapturedError::msg("Could not save the tag."))
}
```

The rules, all demonstrated in `src/db/note.rs`:

- **`session_user.record_id()` for the owner, never an argument.** The client
  can send anything.
- **Filter reads by owner too.** A read by id without `WHERE owner = $user` lets
  anyone read any row whose id they can guess — and ids are in URLs. Return
  `None` for "not yours", so the answer does not confirm the id exists.
- **Bind, never format.** `format!` into SurrealQL is an injection.
- **`#[get]` for reads, `#[post]` for writes.**
- **Unit-test the validation.** It is a plain function; see the tests at the
  bottom of `note.rs`.

New endpoints are guarded by default. Mark one `#[g3_auth::public]` (above
its `#[get]` or `#[post]`) only if it must answer a signed-out visitor, and
say why in a comment; the test in `src/auth/session.rs` pins the list.

## 4. What a change makes stale

Screens read through the client cache, so a mutation has to say what it
changed. In `src/data_change.rs`, add a variant and the reads it affects:

```rust
pub enum DataChange {
    Notes,
    /// A tag was created or deleted (`create_tag`, `delete_tag`).
    Tags,
}

// in `invalidate`:
DataChange::Tags => invalidate_cached(list_tags),
```

A read left off a list shows stale data until its screen reopens; an extra
one only costs a request, so err toward listing it.

## 5. The route

In `src/app.rs`, add a variant and say how it arrives. A pushed page goes in
the second `#[layout(AppShell)]` block:

```rust
#[transition(layer = stack_page)]
#[route("/tags")]
Tags {},
```

Pick the layer from [navigation.md](navigation.md): `layer = stack_root` for a
tab, `layer = stack_page` for a page above one, `layer = sheet` for a task in
`#[layout(SheetShell)]`. If its motion is not obvious from the layer, assert
what you meant in `transition_tests` at the bottom of the file:

```rust
#[test]
fn the_tag_list_pushes_over_settings() {
    assert_eq!(Route::Settings {}.transition_to(&Route::Tags {}), NavigationTransition::Forward);
    assert_eq!(Route::Tags {}.transition_back(), NavigationTransition::Backward);
}
```

Give it a fallback in the `match` in `src/components/shell/back_button.rs`,
for when the page is opened directly with nothing behind it.

## 6. The screen

`src/components/tags/tag_list.rs`. Built from g3-ui components only, with no
custom CSS. The component catalog is [g3-ui.md](g3-ui.md).

```rust
use crate::{
    components::{BackButton, shared::LoadFailed},
    db::{list_tags, record_key},
};
use dioxus::prelude::*;
use dioxus_icons::lucide::Tag as TagIcon;
use g3_cache::use_cached;
use g3_ui::{Content, ContentWidth, EmptyState, Header, Item, List, ListVariant, Spinner};

/// A pushed page: it renders inside `AppShell` but supplies its own header.
#[component]
pub fn Tags() -> Element {
    // Every hook first. Cached on the device: the list shows at once and
    // refetches behind it, and `DataChange::Tags` marks it stale.
    let tags = use_cached(list_tags, ());

    let header = rsx! {
        Header { title: "Tags", start: rsx! { BackButton {} } }
    };

    // Borrow from the read guard; no clone of the list per render.
    let tags_read = tags.read();
    let body = match tags_read.as_ref() {
        None => rsx! { Spinner { center: true } },
        Some(Err(_)) => rsx! { LoadFailed {} },
        Some(Ok(rows)) if rows.is_empty() => rsx! {
            EmptyState {
                title: "No tags yet",
                icon: rsx! { TagIcon { size: 40 } },
                "Tags you add show up here."
            }
        },
        Some(Ok(rows)) => rsx! {
            List { variant: ListVariant::Raised,
                for tag in rows {
                    Item { key: "{record_key(&tag.id)}", label: tag.label.clone() }
                }
            }
        },
    };

    rsx! {
        {header}
        Content { width: ContentWidth::Readable, {body} }
    }
}
```

The patterns that keep screens out of trouble (the full guide is
[dioxus/patterns.md](dioxus/patterns.md)):

- **Hooks at the top, before any early return.** A hook inside a `match` arm
  or after a `return` panics or goes stale the first time the branch changes.
- **Handle every state:** loading (`Spinner`), error (`LoadFailed`), empty
  (an `EmptyState` saying what will appear), then the content.
- **A prop read inside a hook is a `ReadSignal<T>`**, so the hook reruns when
  it changes: `fn TagDetail(id: ReadSignal<String>)` with
  `use_cached(get_tag, (id(),))`. No `use_reactive!`.
- **Never hold a signal borrow across `.await`.** Read into a local first.
  `clippy.toml` lints for it.
- **Navigate with `animated_navigate`**, and go back with `BackButton`.
- **After a mutation**, `app_state.changed(DataChange::Tags)`, then
  `use_toast()` for feedback. For instant feedback, `update_cached` first,
  as the notes list does when pinning.
- **A form's draft lives in its own component**, seeded from props, so typing
  re-renders only the form (see `NoteForm`).

Wire it up: `mod tag_list; pub use tag_list::*;` in
`src/components/tags/mod.rs`, `mod tags; pub use tags::*;` in
`src/components/mod.rs`, and import `Tags` in `src/app.rs`. Link to it from
Settings with an `Item { label: "Tags", detail: ItemDetail::Show, onclick: move |_| animated_navigate(Route::Tags {}) }`.

## 7. Check it

```bash
just check    # web, server, and mobile builds
just test     # includes the transition and account-deletion tests
just lint-strict
```

`just check` matters more than it looks: `cargo check` alone builds only the web
feature set, so a server-only mistake stays hidden until CI.

Then look at it: `just dev`, a 390×844 viewport, then 1440×900 for the desktop
rail. If the flow is one a user depends on, extend `tests/ui/smoke.spec.mjs`.

---

## Removing the example

When you start on your own domain:

1. Delete `src/components/notes/`, `src/db/note.rs`, and
   `database/schema/note.surql`, and their `mod` lines.
2. Remove the `Notes`, `NoteDetail`, `NewNote`, and `EditNote` routes and their
   transition tests, and point the tab bar in `AppShell`, the splash, sign-in,
   and `BackButton` fallbacks at your first screen. The compiler lists every
   place.
3. Remove `DELETE note ..` from `DELETE_ACCOUNT_QUERY`, and `DataChange::Notes`
   from `src/data_change.rs`.
4. Replace `database/seed/00_demo.surql` and `tests/ui/smoke.spec.mjs`.
