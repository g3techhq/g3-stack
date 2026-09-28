# Navigation and transitions

How screens connect, how they animate, and how Back works on every platform.
The library doing the work is
[`g3-route-transitions`](https://github.com/g3techhq/g3-route-transitions) 0.4
with the `native-back` feature; its README is the full reference.

---

## The idea

Animation is a property of the **route**, not of the component. Every screen's
entrance is declared once, on the `Route` enum in `src/app.rs`, and the library
works out the right transition for any pair of routes, including the reverse
on Back. No component in this template contains animation code, and none of
yours should.

```rust
#[derive(Debug, Clone, Routable, PartialEq, RouteTransitions)]
pub enum Route {
    #[transition(layer = stack_root, history = replace)]
    #[route("/notes?:filter")]
    Notes { filter: Option<NotesFilter> },

    #[transition(layer = stack_page)]
    #[route("/notes/:id")]
    NoteDetail { id: String },

    #[transition(layer = sheet)]
    #[route("/notes/:id/edit")]
    EditNote { id: String },
}
```

Each transition renders in iOS or Material motion depending on the current
mode. It uses the View Transitions API; without it, or with reduced motion,
routes change instantly.

## The three pieces

1. **Route metadata** decides *which* transition runs.
2. **Snapshot regions** decide *what moves*. g3-ui places most of them
   (`AppWrapper` is the overlay, `TabLayout` the base, the rail persistent);
   `AppShell` adds a `RouteTransitionPage` around each page.
3. **Animated navigation** captures the old page before the route changes:
   `animated_navigate(route)` and `animated_back_or_navigate(fallback)`.
   `navigator().push(..)` and `go_back()` change the route first, so they skip
   the animation.

---

## Layers: what a route is

Pick one.

| Layer | For | Layout here | Arrives by |
| --- | --- | --- | --- |
| `layer = stack_root` | A tab: Notes, Settings | first `#[layout(AppShell)]` | cross-fade from another tab |
| `layer = stack_page` | A page above a tab: a detail view | second `#[layout(AppShell)]` | push from the right (iOS) or shared axis (Material) |
| `layer = sheet` | A task you finish or abandon: an editor, a picker | `#[layout(SheetShell)]` | rises from the bottom over the dimmed page |
| *(none)* | Anything else: the splash, sign-in | `RootLayout` only | cross-fade |

**Which one?** If you drill into it, `stack_page`. If it is a task you finish
and return from, `sheet`. If it is one of the tabs, `stack_root`.

Pushed pages reuse `AppShell` so the tab bar or rail stays still while only
the page region slides; the page renders its own `Header` with a
`BackButton`. `SheetShell` renders the same navigation with
`AdaptiveNavCompact::Hidden`: on a phone the sheet covers the tabs, on a wide
screen it rises beside the rail.

## Options: how history behaves

| Option | Effect | Use it for |
| --- | --- | --- |
| `history = replace` | A change *within* the same variant is instant and replaces the history entry | A filter or tab in the URL. Tapping "Pinned" should not cross-fade the page, or make Back walk through every filter |
| `history = replace(key = id)` | The same, but only while the listed fields match | A tabbed detail page: switching tabs on one note replaces; opening a different note does not |
| `handoff_from = Route` or `(A, B)` | Arriving from a listed route replaces its history entry | Screens that forward you at once, like the splash and sign-in, so Back skips them |
| `forward_to = Route` or `(A, B)` | Moving to a listed route is `Forward`, and back from it `Backward` | A drill-down between two `stack_page`s, which otherwise cross-fade |
| `peers(group = g, order = field)` | Orders sibling routes, so moving between them slides left or right | Wizard steps; `key = field` scopes the group to one entity |

`history = replace` combined with `peers` slides *and* replaces history, which
is what a segmented control over sliding content wants.

**Back looks only at the layer being left**: leaving a `sheet` dismisses it,
leaving a `stack_page` slides backward, anything else cross-fades. Give a
drill-down destination `layer = stack_page` or `sheet` when Back should mirror
the way it came.

## Navigating

```rust
use g3_route_transitions::animated_navigate;

// An event handler may return the future; Dioxus spawns it.
onclick: move |_| animated_navigate(Route::NewNote {}),

// Elsewhere, spawn it.
spawn(animated_navigate(Route::NoteDetail { id }));
```

- **`animated_navigate(route)`** applies the rules. It does nothing if `route`
  is already current.
- **Back is `BackButton {}`** (`src/components/shell/back_button.rs`) in the
  `Header`'s `start` slot. It calls `animated_back_or_navigate` with a
  fallback per route, used only when there is no history to pop (a link
  opened directly, a reload). Extend its `match` when you add a pushed page
  or a sheet.
- **`navigator().replace` is still right** when the current entry must not be
  returnable to: after deleting the thing the page shows, or the account.
- All of these must run beneath `Router::<Route>`.

## Test what you meant

The rules are declarative, so they are testable without a browser. A route
whose motion is not obvious from its layer gets an assertion in
`transition_tests` at the bottom of `src/app.rs`:

```rust
#[test]
fn the_editor_rises_as_a_sheet() {
    let detail = Route::NoteDetail { id: "abc".into() };
    let editor = Route::EditNote { id: "abc".into() };
    assert_eq!(detail.transition_to(&editor), NavigationTransition::PresentSheet);
    assert_eq!(editor.transition_back(), NavigationTransition::DismissSheet);
}
```

The methods are `transition_to(&other)`, `transition_back()` and
`replaces_history(&other)`. `NavigationTransition` has `Forward`, `Backward`,
`PresentSheet`, `DismissSheet`, `CrossFade` and `None`.

## Route-enum layout syntax

The macro is strict about it:

- `#[layout(X)]` opens a layout and `#[end_layout]` closes it.
- A layout never closed runs to the end of the enum. A trailing
  `#[end_layout]` with nothing after it is a **parse error**, which is why the
  last layout in the template is left open.

---

## Back on every platform

| Platform | Back comes from | What happens |
| --- | --- | --- |
| Web | The browser's Back and Forward | `use_browser_history_transitions`, in `ThemedShell`, animates them like the app's own |
| Android | System Back gesture or button | `use_native_back_navigation`, in `RootLayout`, runs the same animated pop. At the root of history Android leaves the app, as usual |
| iOS | Left-edge swipe | The same animated pop. The swipe does nothing at the root |

`native-back` on `g3-route-transitions` and `back-button` on
`g3-native-plugins` are both already enabled.

### Sheets and system Back

A g3-ui `Sheet` or `Modal` is not a route, so the router knows nothing about
it. Back while one is open should close it rather than leave the page:

1. **Keep interception on while a sheet is open**, even at the root of
   history, by switching the hook in `RootLayout`:

   ```rust
   use_native_back_navigation_with_interception::<Route>(g3_ui::open_sheet_count() > 0);
   ```

2. **Close the topmost sheet when Back arrives.** The native plugin raises a
   cancelable `g3nativeback` event on `window`; call `event.preventDefault()`
   and close the sheet.

The template does not ship this, because its only sheet-like surfaces are
`layer = sheet` routes, which Back already handles. Add it the day you put a
`Sheet` on a tab.

---

## Per-screen state lives in the URL

A filter, a tab, a sort order, a search term: put it in the route.

```rust
#[transition(layer = stack_root, history = replace)]
#[route("/notes?:filter")]
Notes { filter: Option<NotesFilter> },
```

It survives a refresh, Back walks out of the screen rather than through every
filter, and a filtered view is something you can link to. The type needs
`Display` and `FromStr`; `strum`'s derives give you both, as `NotesFilter`
shows. `Option<..>` keeps the plain `/notes` URL valid.

The control showing the filter follows the route rather than owning it:
`NotesToolbar` sets `defer_selection` on its `SegmentGroup`, reports picks
through `onchange`, and an effect moves it when the route changes some other
way (browser Back).

**Never parse a path string into `Route`** to decide anything. The catch-all
`#[redirect("/:.._segments", ..)]` parses every path as the splash, `/api/`
included. Match on the `Route` you already have.

`AppState` is for what spans screens; see [architecture.md](architecture.md).

## Mode and platform

g3-ui's mode (which look) and g3-route-transitions' `Platform` (which motion)
are separate globals that must agree. Change the appearance through
`AppState::set_appearance` or `state::apply_mode`, which set both.
