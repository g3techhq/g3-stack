# Navigation and transitions

How screens connect, how they animate, and how Back works on every platform.
The library doing the work is
[`g3-route-transitions`](https://github.com/g3techhq/g3-route-transitions); see
it running at [g3ui.g3tech.net/transitions](https://g3ui.g3tech.net/transitions).

---

## The idea

Animation is a property of the **route**, not of the component. Every screen's
entrance is declared once, on the `Route` enum in `src/app.rs`, and the library
works out the right animation for any pair of routes — including the reverse
on Back. No component in this template contains animation code, and none of
yours should.

```rust
#[route_transitions]
#[derive(Debug, Clone, Routable, PartialEq)]
pub enum Route {
    #[transition(root, replace)]
    #[route("/notes?:filter")]
    Notes { filter: Option<NotesFilter> },

    #[transition(pushed)]
    #[route("/notes/:id")]
    NoteDetail { id: String },

    #[transition(cover)]
    #[route("/notes/:id/edit")]
    EditNote { id: String },
}
```

Each animation renders in iOS or Material motion depending on the current mode.

---

## Layers: what a route is

Pick exactly one.

| Layer | For | Arriving | Leaving |
| --- | --- | --- | --- |
| `root` | A bottom-tab destination | Cross-fade from another root | Cross-fade |
| `pushed` | A full-screen page above a tab: a detail view | Slides in from the right (iOS parallax, or Material shared axis) | Slides out to the right |
| `cover` | A sheet or modal-like task: an editor, a picker, a composer | Rises from the bottom | Drops downward |
| `morph` | A page that grows out of a card on a `base` route | Scales up from the card | Shrinks back |
| `base` | Anything else. The default; you rarely write it | Fade | Fade |

**Which one?** If it replaces the tab bar and you drill into it, `pushed`. If it
is a task you finish or abandon and return to where you were, `cover`. If it is
one of the tabs, `root`.

## Modifiers: how history behaves

| Modifier | Effect | Use it for |
| --- | --- | --- |
| `replace` | A change *within* the same variant does not animate and replaces the history entry | A filter, segment, or search term in the URL. Tapping "Pinned" should not cross-fade the page or make Back walk through every filter |
| `replace(key = id)` | Same, but only when the listed fields match | A tabbed detail page: switching tabs on one note replaces; opening a different note does not |
| `replaces = Route` or `(A, B)` | Arriving from a listed route replaces its history entry | Screens that immediately forward you, like a splash or sign-in, so Back skips them. Also a sheet opened from another sheet |
| `forward = Route` or `(A, B)` | Declares a drill-down between two `pushed` routes, so both directions slide the right way | `ListDetail` → `ItemDetail` |
| `push(group = g, order = field)` | Orders peer routes, so moving between them slides left or right | Wizard steps: `#[transition(cover, push(group = setup, order = step))]` |
| `push(group = g, key = field, order = field)` | Scopes the group to one entity | Steps of *this* game's setup, not another's |

`replace` combined with `push` slides *and* replaces history — what a segmented
control over sliding content wants.

**Never point `forward` at a `cover` route.** `forward` wins over the
destination's own layer, so the sheet would slide in sideways instead of
rising. It is a mistake easy to make and easy to miss, which is why the
template's `NoteDetail` has a comment where the tempting `forward = EditNote`
would go.

## Navigating

```rust
use g3_route_transitions::{animated_navigate, animated_go_back};

// Forward. Picks the animation and whether to push or replace history.
spawn(animated_navigate(Route::NoteDetail { id }));

// Back, with a fallback for when there is no history (a deep link, a refresh).
spawn(async move { animated_go_back(Route::Notes { filter: None }).await });
```

- **Use `animated_navigate`, not `navigator.push`.** The animation needs a
  snapshot of the outgoing page, taken before the route changes. `push`
  changes it first.
- **Use `animated_go_back`, not `navigator.go_back`,** for the same reason. The
  fallback is used only when there is nothing to pop.
- **`navigator.replace` is still right** when the current entry must not be
  returnable to — after deleting the thing the page shows, or after deleting
  the account.
- The template's `BackButton` (`src/components/shell/back_button.rs`) picks a
  sensible fallback per route. Extend its `match` when you add a pushed page or
  sheet.

## Test what you meant

Because the rules are declarative, they are testable without a browser. Every
route you add should get an assertion in `transition_tests` at the bottom of
`src/app.rs`:

```rust
#[test]
fn opening_a_tag_pushes_and_back_reverses_it() {
    let tags = Route::Tags {};
    let tag = Route::TagDetail { id: "t1".into() };
    assert_eq!(tags.transition_to(&tag), NavigationAnimation::PushLeft);
    assert_eq!(tag.transition_to(&tags), NavigationAnimation::PushRight);
}
```

The methods are `transition_to(&other)`, `transition_back()`, and
`replaces_history(&other)`. `NavigationAnimation` has `None`, `Fade`,
`PushLeft`, `PushRight`, `CoverUp`, `UncoverDown`, `MorphIn`, and `MorphOut`.

---

## Layouts and snapshots

Two layouts in `src/app.rs` exist for the animation's sake:

- **`AppShell`** wraps the tab routes in one `RouteTransitionPage`, so switching
  tabs keeps the header and tab bar mounted.
- **`PushedPageLayout`** wraps pushed routes in a `RouteTransitionPage`, so the
  header, body, and (absent) tab bar move as one image instead of three that
  can slide over each other.

`cover` routes need neither: g3-ui's `AppWrapper` already marks the shell as the
cover snapshot. Put a new pushed route inside `#[layout(PushedPageLayout)]`, a
new tab inside `#[layout(AppShell)]`, and a new sheet between the two.

Route-enum layout syntax, which the macro is strict about:

- `#[layout(X)]` opens a layout and `#[end_layout]` closes it.
- A layout never closed runs to the end of the enum. A trailing `#[end_layout]`
  with nothing after it is a **parse error**, which is why the last layout in
  the template is left open.

---

## Back on every platform

| Platform | Back comes from | What happens |
| --- | --- | --- |
| Web | The browser | The browser pops history; the transition runs for in-app back buttons |
| Android | System Back gesture or button | `use_native_back_navigation` runs the same animated pop as the in-app button. At the root of history, Android's normal behavior (leave the app) is preserved |
| iOS | Left-edge swipe | Same animated pop. The swipe does nothing at the root |

This is one line in `RootLayout`:

```rust
use_native_back_navigation::<Route>();
```

It needs the `native-back` feature on `g3-route-transitions` and the
`back-button` feature on `g3-native-plugins`, both already enabled.

### Sheets and system Back

A g3-ui `Sheet` or `Modal` is not a route, so the router knows nothing about it.
Back while one is open should close it rather than leave the page. Two parts:

1. **Keep interception on while a sheet is open**, even at the root of history,
   by switching the hook in `RootLayout`:

   ```rust
   use_native_back_navigation_with_interception::<Route>(g3_ui::open_sheet_count() > 0);
   ```

2. **Close the topmost sheet when Back arrives.** The native plugin raises a
   cancelable `g3nativeback` event on `window`, and every open dismissible sheet
   renders a hidden `[data-g3-sheet-dismiss]` control. A listener installed
   once:

   ```js
   window.addEventListener("g3nativeback", (event) => {
     const dismiss = [...document.querySelectorAll(".g3-sheet.g3-sheet-open [data-g3-sheet-dismiss]")].pop();
     if (dismiss) {
       event.preventDefault();
       dismiss.click();
     }
   });
   ```

The template does not ship this, because its only sheet-like surfaces are
`cover` routes, which Back already handles. Add it the day you put a `Sheet` on a
root screen.

---

## Per-screen state lives in the URL

A filter, a tab, a sort order, a search term: put it in the route.

```rust
#[transition(root, replace)]
#[route("/notes?:filter")]
Notes { filter: Option<NotesFilter> },
```

It survives a refresh, Back moves through it (or, with `replace`, skips it), and
a filtered view is something you can link to. The type needs `Display` and
`FromStr`; `strum`'s derives give you both, as `NotesFilter` shows. `Option<..>`
keeps the plain `/notes` URL valid.

`AppState` is for what spans screens — see [architecture.md](architecture.md).
