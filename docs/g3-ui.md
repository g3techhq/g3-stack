# g3-ui reference

The components the template builds with, and their props, as of `g3-ui` 0.4.4
with the `transitions` feature. Written so you (or your coding agent) can build
a screen without guessing. The full reference is
[docs.rs/g3-ui](https://docs.rs/g3-ui), and the
[playground](https://g3ui.g3tech.net/) shows each component live in both modes.

**g3-ui is where UI starts in this stack.** Before writing a `div` with
classes, look for the component below that already is that thing. It carries
the iOS and Material looks, dark mode, the desktop layout and ARIA.

---

## Rules that apply to every component

- **Optional props are `Option<T>`, and `rsx!` wraps them for you.** Write
  `title: "Notes"`, not `title: Some("Notes".to_string())`. String props
  accept `&str`.
- **Stateful components take a `Signal` and write it themselves.** Pass
  `value: title`, not a value plus an `onchange` that sets it. `onchange` is
  for a side effect on top (saving). Leave the signal out and the component
  keeps its own state.
- **A control that follows something else** (a filter in the URL) sets
  `defer_selection`: it only reports picks through `onchange` and leaves its
  `value` to you. See `NotesToolbar`.
- **Every component takes `class`**, and most take **`mode`** to force iOS or
  Material on one instance. You rarely need either.
- **Spacing is built in.** Use `Stack { gap: Space::.. }` for vertical rhythm
  rather than a wrapper `div` with margins.
- Names are unprefixed (`Button`, `List`). Import them explicitly
  (`use g3_ui::{Button, Card}`); rename on import (`List as G3List`) if one
  clashes with a type of your own.
- Slots are `start` / `end` everywhere (buttons, items, cards, the header).
- `to: Route` on `Button`, `Card`, `Item` and `NavItem` makes it a link. In
  this template, navigate with `onclick` and `animated_navigate` instead, so
  the transition runs (see [navigation.md](navigation.md)).

---

## Page structure

| Component | Key props | Notes |
| --- | --- | --- |
| `AppWrapper` | `theme`, `mode`, `text_selection` | Once, in `ThemedShell` (`src/app.rs`). Theme as CSS variables, the stylesheet, the host for toasts and alerts, and the transition overlay region. |
| `TabLayout` | `route_transition_base: bool` | The frame. `AppShell` and `SheetShell` (`components/shell/app_shell.rs`) are already assembled; screens never render one. |
| `AdaptiveNav` / `NavItem` | `compact: AdaptiveNavCompact`; `label`, `icon`, `selected`, `group`, `onclick` | Bottom tabs on a phone, a rail from 48rem. `NavItemGroup::Secondary` sits at the foot of the rail. Lives in `ShellNav`. |
| `Header` | `title`, `start`, `end`, `toolbar`, `title_content`, `title_end` | A tab's header comes from `AppShell`; a pushed page or sheet renders its own, with `start: rsx! { BackButton {} }`. `toolbar` holds a `SegmentGroup` (under the title on a phone, inline from 64rem). |
| `Content` | `width: ContentWidth::{Full, Readable, Wide}`, `padding`, `on_refresh` + `refreshing`, `fab` | The scroll container. `Readable` for forms and text, `Wide` for grids. Wraps its children in suspense and error boundaries. |

```rust
rsx! {
    Header { title: "Note", start: rsx! { BackButton {} } }
    Content { width: ContentWidth::Readable,
        Stack { /* the screen */ }
    }
}
```

## Layout and text

| Component | Key props | Notes |
| --- | --- | --- |
| `Stack` | `horizontal`, `gap: Space::{None, Xs, Sm, Md, Lg, Xl}`, `align: StackAlign`, `justify`, `wrap` | The default way to lay out children. Default gap is `Md`. |
| `Grid` | `columns`, `wide_columns`, `gap`, `wide_gap` | Columns that change at the 48rem shell width. |
| `Text` | `variant: TextVariant::{Title, Heading, Body, Caption, Label, Overline}`, `tone: TextTone::{Primary, Secondary, Tertiary}`, `color: Color`, `truncate` | All copy. Supporting text is `Text { tone: TextTone::Secondary, .. }`. |
| `Divider`, `ListHeader` | | Separators and headed groups inside a `List`. |

## Content

| Component | Key props | Notes |
| --- | --- | --- |
| `Card` | `title`, `subtitle`, `start`, `end`, `media`, `variant: CardVariant::{Raised, Flat, Filled}`, `onclick`, `selected` | A block of related content or a form. |
| `List` | `variant: ListVariant::{EdgeToEdge, Raised, Flat, Filled}`, `lines: ListLines::{Full, Inset, None}` | A group of `Item`s. Raised + inset lines is the settings look. |
| `Item` | `label`, `description`, `overline`, `metadata`, `start`, `end`, `onclick`, `detail: ItemDetail::{Auto, Show, Hide}`, `selected`, `checked` | A row. `ItemDetail::Show` draws the chevron for a row that opens something; `Hide` for a row that acts in place (Sign out). |
| `SwipeItem` / `SwipeAction` | `start_actions`, `end_actions`, `start_behavior`, `on_activate`; `color`, `aria_label`, `onclick` | Swipe a row to reveal actions (pin a note). |
| `EmptyState` | `title` (required), `icon`, `action`, `color` | Nothing to show yet, and failures (`LoadFailed` in `components/shared/resource_states.rs`). |
| `Avatar` | `name` (required), `src`, `size` | People rows. Initials come from `name`. |
| `Badge`, `Chip` | `color`; `selected`, `onclick` | Counts and statuses; filter and tag chips. |
| `Shelf` | `title`, `end`, `snap` | A horizontal scroller of cards. |
| `ReorderList` / `ReorderItem` / `ReorderHandle` | `onreorder: (from, to)`; `index`; `label` | Drag to reorder. |

## Actions and forms

| Component | Key props | Notes |
| --- | --- | --- |
| `Button` | `fill: ButtonFill::{Solid, Outline, Clear}`, `color: Color::{Accent, Neutral, Success, Warning, Danger}`, `size: ButtonSize`, `expand: ButtonExpand::{Block, Full}`, `loading`, `disabled`, `start`, `end`, `aria_label`, `onclick` | `loading: saving()` while a request runs. Icon-only buttons need `aria_label`. A destructive action is `fill: Outline, color: Danger`. |
| `Input` | `label`, `value: Signal<String>`, `input_type: InputType`, `placeholder`, `helper`, `error`, `maxlength`, `debounce_ms`, `onchange` | One field with its label, helper and error. The label is the accessible name, so Playwright finds it with `getByLabel`. |
| `TextArea` | `label`, `value`, `rows`, `maxlength`, `helper`, `error` | Longer text (a note's body). |
| `Searchbar` | `value`, `placeholder`, `debounce_ms`, `onchange`, `on_submit`, `end` | Search boxes; `onchange` fires after the debounce. |
| `SegmentGroup` / `SegmentButton` | `value: Signal<T>`, `onchange`, `defer_selection`, `aria_label`, `scrollable`; `value: T` | Generic over the value, so options are enums (`NotesFilter`, `ComponentMode`). |
| `Toggle`, `Checkbox` | `checked: Signal<bool>`, `label`, `aria_label`, `helper`, `onchange` | Settings switches. |
| `Select`, `RadioGroup` / `Radio` | `value`, `options` / children, `label` | Generic over the value. |

## Overlays and feedback

| Component | Key props | Notes |
| --- | --- | --- |
| `use_toast()` | `.success(msg)`, `.error(msg)`, `.show(msg)` | Feedback after an action. `AppWrapper` hosts them; never mount a `Toast` yourself. |
| `use_alert()` | `.confirm(title, message).await`, `.show(AlertOptions)`, `.prompt(..)` | A question from a handler. `confirm_destructive` in `components/shared/` asks with a danger-colored button. |
| `use_action_sheet()` | `.show(ActionSheetOptions).await` | A short list of choices. |
| `BottomSheet` | `open: Signal<bool>` (required), `title`, `detents`, `on_dismiss`, `backdrop` | Pickers and short flows that are not their own route. A floating panel from 48rem. |
| `Modal` | `open`, `title`, `actions`, `size` | A focused dialog. |
| `Spinner` | `center`, `size`, `label` | Loading, when there is no skeleton for the shape. |
| `Skeleton` | `shape: SkeletonShape`, `width` | Loading placeholders. |
| `Refresher` | `refreshing`, `on_refresh` | Pull to refresh (the notes list), or `Content { on_refresh }` for a whole page. |
| `InfiniteScroll` | `on_load` (required), `loading` (required), `complete` | Paged results. |

**A sheet route, a `BottomSheet`, or an alert?** A `Route` with
`layer = sheet` is a whole page rendered by `SheetShell`: use it when the task
should survive a refresh, be linkable, or be closed by Back (the note
editor). A `BottomSheet` is an overlay inside a page, for a quick choice that
belongs to the screen that opened it. A yes/no question is `use_alert`.

---

## Theme

A `Theme` is a set of color tokens. `AppWrapper` writes each as a
`--g3-color-*` custom property, so anything you style yourself references
these rather than a literal:

| Field | Custom property | Is |
| --- | --- | --- |
| `accent` / `on_accent` | `--g3-color-accent` / `--g3-color-on-accent` | Primary buttons, selected controls, focus rings, links; text drawn on them |
| `text` / `text_secondary` / `text_tertiary` | `--g3-color-text` / `-text-secondary` / `-text-tertiary` | Body copy, descriptions, placeholders |
| `bg` / `bg_secondary` | `--g3-color-bg` / `--g3-color-bg-secondary` | The page, and a recessed ground |
| `card` | `--g3-color-card` | Cards, list rows, headers, tab bars |
| `surface` | `--g3-color-surface` | Dialogs and popovers |
| `control` | `--g3-color-control` | Chips, avatars, unselected controls |
| `border` | `--g3-color-border` | Hairlines and separators |
| `success` / `warning` / `danger` | `--g3-color-success` / `-warning` / `-danger` | Status |
| `shadow` | `--g3-color-shadow` | Elevation |
| `color_scheme` | `color-scheme` | `"light"`, `"dark"` or `"light dark"` |

`Theme::default_light()` and `Theme::default_dark()` are the presets.
`.with_accent("#16a34a")` swaps the accent; struct-update syntax
(`Theme { card: "#fff".into(), ..Theme::default_light() }`) replaces anything
else. See [styling.md](styling.md).

## Modes

`ComponentMode::Ios` or `ComponentMode::Md`. The template drives it from the
user's saved preference through `AppState::set_appearance`, which also
switches g3-route-transitions' motion so the two never disagree. Never call
`g3_ui::set_mode` on its own.

## Upgrading g3-ui

The 0.3 → 0.4 renames (`Body` → `Content`, `Navbar` → `TabLayout`,
`ButtonStyle` → `ButtonFill`, `Field` → `Input`/`TextArea`, `--color-*` →
`--g3-color-*` ..) are in the crate's CHANGELOG. When a new version lands,
read its CHANGELOG, bump the version, and let `just check` list the call
sites.
