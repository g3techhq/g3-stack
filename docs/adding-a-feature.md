# Adding a feature

The notes feature is the reference implementation. This walks through building
the equivalent from scratch, in the order the pieces depend on each other:
table, Rust types, server functions, route, screen.

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

New endpoints are guarded by default. Add a path to `is_unsecured_path` in
`src/auth/session.rs` only if it must answer a signed-out visitor.

## 4. The route

In `src/app.rs`, add a variant and say how it arrives. A pushed page goes inside
`#[layout(PushedPageLayout)]`:

```rust
#[transition(pushed)]
#[route("/tags")]
Tags {},
```

Pick the layer from [navigation.md](navigation.md): `root` for a tab, `pushed`
for a page above one, `cover` for a sheet. Then assert what you meant in
`transition_tests` at the bottom of the file:

```rust
#[test]
fn the_tag_list_pushes_over_settings() {
    assert_eq!(Route::Settings {}.transition_to(&Route::Tags {}), NavigationAnimation::PushLeft);
    assert_eq!(Route::Tags {}.transition_back(), NavigationAnimation::PushRight);
}
```

If the page has a back button, give it a sensible fallback in the `match` in
`src/components/shell/back_button.rs`.

## 5. The screen

`src/components/tags/tag_list.rs`. Built from g3-ui components only — no custom
CSS. The component catalog is [g3-ui.md](g3-ui.md).

```rust
use crate::{components::PageShell, db::{list_tags, record_key}, state::AppState};
use dioxus::prelude::*;
use g3_ui::{Button, ButtonStyle, Card, Item, List, Spinner};

/// A pushed page covers the tab bar, so it renders its own shell.
#[component]
pub fn Tags() -> Element {
    let app_state = use_context::<AppState>();

    // Every hook first. `data_version` is read only to subscribe, so this
    // refetches after a mutation made on any other screen.
    let mut tags = use_resource(move || {
        let _ = (app_state.data_version)();
        async move { list_tags().await }
    });

    let body = match &*tags.read() {
        None => rsx! { Spinner { center: true } },
        Some(Err(error)) => rsx! {
            Card { title: "Could not load your tags",
                p { class: "g3-message-text g3-message-text-muted", "{error}" }
                Button { style: ButtonStyle::Outline, onclick: move |_| tags.restart(), "Try again" }
            }
        },
        Some(Ok(rows)) if rows.is_empty() => rsx! {
            Card { title: "No tags yet",
                p { class: "g3-message-text g3-message-text-muted", "Tags you add show up here." }
            }
        },
        Some(Ok(rows)) => rsx! {
            List { inset: true,
                for tag in rows.clone() {
                    Item { key: "{record_key(&tag.id)}", label: tag.label }
                }
            }
        },
    };

    rsx! {
        PageShell { title: "Tags", {body} }
    }
}
```

The patterns that keep screens out of trouble:

- **Hooks at the top, unconditionally.** A `use_resource` inside a `match` arm
  panics the first time the arm changes.
- **Handle all three states:** loading (`Spinner`), error (a `Card` with a retry),
  and empty (a `Card` saying what will appear). A resource that errors and shows
  a spinner forever is the most common bug in a new screen.
- **Never hold a signal borrow across `.await`.** Read into a local first.
  `clippy.toml` lints for it.
- **A prop that feeds a resource** needs `use_reactive!`, or the resource keeps
  using the value it mounted with: `use_resource(use_reactive!(|id| async move { get_tag(id).await }))`.
- **Navigate with `animated_navigate`**, and go back with `animated_go_back`.
- **After a mutation**, `app_state.bump_data()`, then `app_state.show_toast(..)`
  for feedback.

Wire it up: `mod tag_list; pub use tag_list::*;` in
`src/components/tags/mod.rs`, `mod tags; pub use tags::*;` in
`src/components/mod.rs`, and import `Tags` in `src/app.rs`. Link to it from
Settings with an `Item { kind: ItemKind::Button, onclick: .. }` that calls
`animated_navigate(Route::Tags {})`.

## 6. Check it

```bash
just check    # web, server, and mobile builds
just test     # includes the transition and account-deletion tests
just lint
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
3. Remove `DELETE note ..` from `DELETE_ACCOUNT_QUERY`.
4. Replace `database/seed/00_demo.surql` and `tests/ui/smoke.spec.mjs`.
