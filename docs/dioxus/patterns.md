# Dioxus patterns for g3 apps

How the g3 apps use Dioxus 0.7, and the mistakes that cost us time. The rest
of `docs/dioxus/` is the framework's own architecture notes, copied from
upstream; this file is ours. When the two disagree about how *this* app should
be written, this file wins.

Pinned: `dioxus`, `dioxus-fullstack` and `manganis` `=0.7.9`, matching the `dx`
CLI. Your training data probably describes 0.5 or 0.6; server functions,
assets, signals and props all changed since.

---

## Components and props

### Props that feed hooks: `ReadSignal<T>`

A plain prop is a snapshot. A hook that reads it (`use_effect`, `use_memo`,
`use_resource`, `use_cached`) runs with the value from the first render and
never sees a new one. The old fix was `use_reactive!`; the better one is to
type the prop as a `ReadSignal`:

```rust
// The router, or a parent, still passes a plain String.
#[component]
pub fn NoteDetail(id: ReadSignal<String>) -> Element {
    let note = use_cached(get_note, (id(),));   // follows the id
    ..
}
```

- The parent passes a plain `T`; `#[component]` wraps it. The child owns one
  signal for its whole life and the new value is written into it, marked
  dirty only when it changed (`PartialEq`), so equal re-renders cost nothing.
- Hooks and effects that read it subscribe like any signal. No
  `use_reactive!`, no dependency tuple to keep in sync.
- A "mirror the prop into a local signal" effect is almost always a
  `ReadSignal` prop plus a `use_memo`, or the local signal is not needed at
  all. The exception is a g3-ui control that must own a `Signal` while
  following the route (`NotesToolbar`): there the effect reads the
  `ReadSignal`, so it reruns on change, and writes the control's signal.
- Use it for large values too: `rows: ReadSignal<Vec<Row>>` is read by
  reference (`rows.read()`) instead of cloned into every closure.
- Keep plain props for values only used while rendering (a label, a flag read
  once in `rsx!`): they re-render the child when they change, which is what
  you want there.

### Props the child writes: `Signal<T>`

A child that changes the parent's state takes the parent's `Signal` (g3-ui's
stateful components do: `Toggle { checked: dark }`). Do not pass a value plus
an `onchange` that sets it.

### Handlers: `use_callback` for what a busy page passes down

`EventHandler<T>` / `Callback<T>` props are compared by identity, and when a
component re-renders, the macro re-points the child's old handler at the new
one (`Callback::__point_to`). A closure written in `rsx!` is a new handler
every render. Usually that is fine. On a page that re-renders often for its
own reasons, and passes handlers to children (a pull-to-refresh, a
load-more), it panicked inside `__point_to`: "RefCell already borrowed", or
with generational-box's `debug_borrows`, a `ValueDroppedError`.

The rule: a handler a page passes as a prop, on a page that re-renders for
reasons of its own, is a `use_callback`, made once above any early return,
that reads what it needs when it runs:

```rust
let load_more = use_callback(move |_: ()| {
    let token = next_page.peek().clone();    // read at call time
    spawn(async move { /* fetch the page, extend the list */ });
});
rsx! { InfiniteScroll { on_load: load_more, .. } }
```

An event handler may return a future (`onclick: move |_| animated_navigate(r)`);
Dioxus spawns it. Call handler props with `.call(value)`.

### Component boundaries follow the data

A component re-renders when anything it reads in its body changes, and a
render re-runs everything in it: every clone, every `format!`, every derived
list. So split a screen along what each part reads, not along how it looks:

- **State that changes often gets its own component.** A text box's draft, a
  drag, an edit in progress. `EditNote` loads the note; `NoteForm` owns the
  draft, seeded once from props, so a keystroke re-renders the form, not the
  loader, and a background refetch cannot overwrite what was typed.
- **A read used by one part lives in that part.** A section that needs its own
  data calls its own `use_cached`; a refetch of one re-renders one section.
- **A read used by several parts is a memo in the parent**, passed down as a
  `ReadSignal`. The parent does not read it in its own body, so it does not
  re-render when it changes.
- **Never call `use_cached` for the same key in two components that mount
  together.** g3-cache does not share an in-flight fetch between hooks, so
  both fetch. Read it once and pass it down.

### Clones: pay on a change or a tap, never on a render

`Signal`, `Memo`, `ReadSignal`, `Resource`, `Cached`, `Callback`, `Store` and
`AppState` are `Copy` handles. Code that clones data on every render almost
always has a handle it could hold instead. In order of preference:

1. **Borrow in render; don't clone out of a read.** A read guard can live for
   the whole body:

   ```rust
   let note_read = note.read();
   let Some(Ok(Some(note))) = note_read.as_ref() else { return rsx! { Spinner {} } };
   rsx! { Text { "{note.body}" } }   // no clone of the note
   ```

   Only props that take an owned value (g3-ui's `label: String`) clone, and
   only that field.

2. **Route and id props are `ReadSignal<String>`.** The component holds a
   `Copy` handle, and every handler reads `id()` when it runs. That removes
   the `let id2 = id.clone()` before each closure.

3. **Shared data is a memo, passed as a `ReadSignal`.** The memo clones when
   the answer changes and notifies only if it differs. `Memo<T>`, `Signal<T>`
   and mapped signals (`memo.map(|m| &m.field)`) all convert to a
   `ReadSignal<T>` prop without copying the value. A `ReadSignal` prop does
   **not** save a clone when the parent has only a plain value.

4. **Derived collections are memos.** The notes list's `shown` memo holds the
   indices the filter keeps, not copies of the notes; it reruns when the list
   or the filter changes, and the rows borrow from the cache.

5. **Rows capture an index; handlers look up at tap time.**

   ```rust
   for &index in shown.read().iter() {
       Item {
           label: all[index].title.clone(),
           onclick: move |_| open(index),
       }
   }
   // where `open` reads `peek_row(&notes, index, |n| record_key(&n.id))`
   ```

   `peek_row` (in `components/shared/resource_states.rs`) reads without
   subscribing.

6. **Closures that capture only `Copy` handles are `Copy`.** Read the id and
   anything else inside the closure instead of capturing clones, and the
   closure can be used by every row with no `.clone()` (`toggle_pin`).

7. **Never call a signal to inspect a collection.** `items().is_empty()`
   clones the whole `Vec` to ask one question. Write `items.read().is_empty()`
   in render, `items.peek().len()` in a handler.

Where it isn't worth it: a label, a flag, a short `Vec<String>`. The goal is
no whole-record or whole-list clones per render.

---

## State

### Hooks

- **Every `use_*` runs every render, in the same order.** All hooks at the top;
  branch on their results after. A hook inside `if`, `match` or after an early
  `return`/`?` panics or reads the wrong slot when the branch changes.
  "Early return" includes the loading branch: a `use_signal` below
  `let Some(x) = .. else { return .. }` is the same bug. Move it up, or into
  the child that renders once the data is there.
- **`use_effect` keeps its first closure.** An effect below an early return
  that is first reached on a later render still runs with the values captured
  then, and never sees newer ones unless it reads them through signals. One
  more reason for hooks above returns.
- **Never hold a signal borrow across `.await`.** Copy the value out first
  (`let id = id();`). `clippy.toml` lints `await_holding_invalid_type` for it.
  That includes a temporary: `save(id.peek().clone(), ..).await` holds the
  `peek` guard until the end of the statement. Bind it on the line before.
- Read with `peek()` when you want the value without subscribing: in event
  handlers, and in an effect that writes the thing it would otherwise read
  (subscribing there loops, and during SSR never returns).
- **An effect depends on exactly what it reads.** To run once per id, read
  the id and `peek()` everything else. That replaces an `initialized` flag.

### Never write a signal in the body

A write during render is a side effect: it dirties whatever reads the signal,
possibly this component, and runs again on every render, on the server too.
Navigating because a read failed, syncing a control to a prop, adopting a
server answer: each goes in an event handler or a `use_effect`.

```rust
// The splash: navigate from an effect, not from the body.
use_effect(move || {
    if let Some(Ok(true)) = *signed_in.read() {
        spawn(animated_navigate(Route::Notes { filter: None }));
    }
});
```

The one sanctioned exception in the template is `AppStateProvider` applying
the signed-in user before SSR, because effects never run on the server and
the first paint must be in the user's theme. It is guarded (writes only when
the answer differs from what is applied) and commented. Do not copy it
elsewhere without the same reason.

### Context: build the signals inside the provider

```rust
let app_state = use_context_provider(AppState::new);   // Signal::new inside `new`
```

`use_context_provider` runs its initializer once, in the provider's scope, so
the signals belong to that scope and live as long as it does. Signals created
in the body and then handed to the provider belong to whatever scope was
current, and are recreated (or dropped) on the next render. A struct made only
of signals derives `Copy`.

### Derived values: `use_memo`

A value computed from signals (a filtered list, a total, a label) goes in a
`use_memo`. It recomputes when an input changes and notifies readers only when
its result changes. Computing it in render re-runs it on every render for any
reason.

```rust
let lit = use_memo(move || NavTab::of(&route.read()));
```

### Stores: nested state edited in parts

`Signal<Struct>` notifies every reader when any field changes. A **store**
tracks each field (and each `Vec`/`HashMap` item) separately:

```rust
#[derive(Store, Clone, Default)]
struct Draft { title: String, body: String, tags: Vec<String> }

let draft = use_store(Draft::default);
draft.title().set("..".into());      // reruns only what read the title
for tag in draft.tags().iter() { .. } // a store per entry
```

- `#[derive(Store)]` generates one lens method per field. Lenses are `Copy`
  and can be props (`Store<T>`, or `ReadStore<T>` for a read-only child).
- Use one when a struct or collection is **edited field by field and read in
  parts** by different components: settings, a large form, a grid of rows
  each edited on its own. Keep a `Signal` for a value replaced whole, and a
  struct of signals (`AppState`) for unrelated app-wide values.

### App-wide state

`AppState` is a `Copy` struct of signals provided once above the `Router`, read
with `use_context::<AppState>()`. It holds what spans screens (the user, the
appearance). Per-screen state (a filter, a tab, a search) goes in the route,
so it survives a refresh and works with Back. Toasts and alerts come from
g3-ui's `use_toast` and `use_alert`, not from `AppState`.

---

## Data

### Reads: `use_cached`, not `use_resource`

```rust
let note = use_cached(get_note, (id(),));   // shows the last answer at once, refetches behind it
match &*note.read() {
    None => rsx! { Spinner { center: true } },
    Some(Err(_)) => rsx! { LoadFailed {} },
    Some(Ok(None)) => rsx! { EmptyState { title: "Note not found" } },
    Some(Ok(Some(note))) => rsx! { Text { "{note.body}" } },
}
```

- Keyed by the function and its arguments; a changed argument is a new read.
  Pass a `ReadSignal` prop's value (`id()`) and it follows the prop.
- The arguments are cloned into the key on every render, so keep the
  component that calls it cheap to re-render.
- A `Cached` is `Copy` but has no `PartialEq`, so it cannot be a prop. Pass a
  memo over it instead.
- `use_resource` is for reads that change with every keystroke (search boxes),
  and for answers that must not come from a cache (the splash's sign-in
  check).
- A read that must answer on the server's first render (the signed-in user)
  uses `use_server_future`, so the HTML arrives already right.

### Writes: show, send, invalidate

```rust
update_cached(list_notes, (), |notes: &mut Vec<Note>| notes[i].pinned = true); // at once
let result = set_note_pinned(id, true).await;                                  // "set to", not toggle
app_state.changed(DataChange::Notes);                                          // refetch what it affected
```

`app_state.changed` hands the change to `data_change.rs`, which maps each
kind of change to the reads it affects. Mutations say what the viewer saw,
never "toggle", so a device acting on stale state cannot undo another
device's change.

`set_cache_owner(None)` empties the cache: call it with `None` only once you
*know* nobody is signed in, never while the session check is still running,
or every launch starts cold.

### Server functions

- `#[get]` for reads, `#[post]` for writes, under `/api/v1/`. A GET query
  string cannot carry a struct; a read taking one is a `#[post]`.
- The owner comes from the session, never from an argument.
- **A server function called during SSR runs without middleware**: the auth
  guard never sees it. Functions acting for the current user still check
  `session_user.anonymous`.
- Bind values with `.bind(..)`; never `format!` a value into SurrealQL.

---

## Rendering

### Hydration

The server renders HTML and the client hydrates it; the two trees must match.

- **Gate calls, never markup.** `#[cfg(feature = "web")] rsx! { .. }` renders
  different trees on each side and silently kills event handlers. Read
  platform or storage state in an effect (client only) and render the same
  thing on both sides first.
- A console `Error deserializing data` or a handler that never fires is a
  hydration mismatch.
- Head elements (`document::Link` ..) go inside the themed shell, not beside a
  component that calls `use_server_future`.

### Routing

- Every page is a `Route` variant; navigation state (tabs, filters) is a route
  field.
- **Never decide access by parsing a path into `Route`.** A catch-all
  `#[redirect("/:..segments", ..)]` parses *every* path, `/api/` included, as
  its target. g3-auth's `#[derive(PublicRoutes)]` matches the route patterns
  instead.
- Navigate with `animated_navigate` and the app's `BackButton`, not the
  navigator directly (see `docs/navigation.md`).

---

## Tooling

- **`dx` pins:** `dioxus*` and `manganis` must match the installed `dx`
  exactly.
- **Do not set `rust-lld` as the linker on Windows**; `dx` builds break.
- **Dev build speed:** `[profile.wasm-dev] debug = 0` halves `dx` rebuilds.
- **Hot-patching does not work on fullstack apps.** A failed patch can leave a
  wasm that imports `env` and a page stuck on "Loading". Stop `dx serve`
  before editing Rust; the recovery is in `docs/troubleshooting.md`.
- **`set_server_url` keeps its first value** (fixed in Dioxus 0.8): the
  server URL of a native build must be known before `dioxus::launch`.
- **A hidden tab never runs `requestAnimationFrame`.** Code that waits a frame
  (route transitions, "after first paint") stalls in a background tab or a
  hidden preview pane.
- **Finding a borrow panic:** add
  `generational-box = { version = "=0.7.10", features = ["debug_borrows"] }`
  to `Cargo.toml` for one build; panics then name where the value was made.
- **Do not run `dx fmt` (0.7.9) on this code.** On a method chain inside a
  closure inside `rsx!` it dropped a `);` and duplicated the next statement.
  Format with `cargo fmt` (which leaves `rsx!` alone) and lay out `rsx!` by
  hand. `.vscode/settings.json` turns off the Dioxus extension's format on
  save (`"dioxus.formatOnSave": "disabled"`) and saves with rustfmt.

---

## Where the framework is documented

- `docs/dioxus/framework-agents.md`: Dioxus's own agent guide.
- `docs/dioxus/architecture/`: how each part works inside. `04-SIGNALS.md`
  (signals, memos, stores), `05-FULLSTACK.md` (server functions, SSR,
  hydration), `09-ROUTER.md`, `07-HOTRELOAD.md`, `08-ASSETS.md`.
- [dioxuslabs.com/learn/0.7](https://dioxuslabs.com/learn/0.7/) for the
  user-facing guide.
