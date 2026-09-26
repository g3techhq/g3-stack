# g3-ui reference

Every component the template can use, with its props, as of `g3-ui` 0.3.0.
Written so you (or your coding agent) can build a screen without guessing. The
full docs are on [docs.rs/g3-ui](https://docs.rs/g3-ui), and the
[playground](https://g3ui.g3tech.net/) shows each one live in both modes.

**g3-ui is where UI starts in this stack.** Before writing a `div` with classes,
look for the component below that already is that thing. It carries the iOS and
Material looks, dark mode, the desktop layout, and ARIA.

---

## Rules that apply to every component

- **Optional props are `Option<T>`, and `rsx!` wraps them for you.** Write
  `title: "Notes"`, not `title: Some("Notes".to_string())`. String props accept
  `&str`.
- **Stateful components take an owned `Signal` and write it themselves.** Pass
  `checked: dark`, not a value plus an `onchange` that sets it. The optional
  `onchange`/`on_change` is for side effects — saving, analytics — on top.
- **Every component takes `class`** for a one-off override and **`mode`** to
  force iOS or Material on that one instance. You rarely need either.
- **Spacing is built in.** Cards carry their own bottom margin, fields their own
  label spacing, and `Body` its own padding. Stacking components needs no
  wrapper `div` with a `gap`.
- Both names are exported: `Button` and `G3Button`. The template imports the
  short names explicitly (`use g3_ui::{Button, Card}`); use the `G3` names if a
  short one collides with a component of your own.

---

## App shell

| Component | Props | Notes |
| --- | --- | --- |
| `AppWrapper` | `theme: Theme`, `mode: ComponentMode`, `disable_text_selection: bool`, `route_transition_root: bool`, `layout: bool` | Once, at the root (`ThemedShell` in `src/app.rs`). Writes the theme as CSS custom properties, links the stylesheet, and is the `g3-app-shell` container that responsive rules query. |
| `Navbar` | `children`, `route_transition_base: bool` | The screen frame. Holds a `Header`, a `Body`, and optionally a `NavbarTabBar`. |
| `Header` | `title: String` (required), `start_button: Element`, `end_button: Element`, `toolbar: Element`, `title_icon: Element` | `toolbar` takes a `SegmentGroup`; it sits under the title on a phone and inline from 64rem. |
| `Body` | `children`, `padding: bool` (default `true`), `has_footer_space: bool` (default `true`), `fab: Element` | The scroll container. Wraps children in a `SuspenseBoundary` (spinner) and an `ErrorBoundary`. |
| `NavbarTabBar` | `children`, `aria_label: String` | Bottom tabs on a phone, a left rail from 48rem. |
| `NavbarTab` | `label: String` (required), `icon: Element`, `selected: bool`, `onclick`, `desktop_placement: NavbarTabDesktopPlacement`, `disabled: bool` | `desktop_placement: Bottom` moves a settings or profile tab to the foot of the rail. |

```rust
Navbar {
    Header { title: "Notes", end_button: rsx! { Button { style: ButtonStyle::Clear, aria_label: "New note", onclick: move |_| {}, Plus { size: 20 } } } }
    Body { Outlet::<Route> {} }
    NavbarTabBar {
        NavbarTab { label: "Notes", selected: true, icon: rsx! { NotebookPen { size: 24 } }, onclick: move |_| {} }
    }
}
```

The template's `AppShell` and `PageShell` (`src/components/shell/`) are this,
already assembled. A new tab goes in `AppShell`; a new pushed page or sheet
wraps itself in `PageShell`.

---

## Actions

| Component | Props | Notes |
| --- | --- | --- |
| `Button` | `onclick` (**required**), `children`, `style: ButtonStyle`, `size: ButtonSize`, `expand: bool`, `disabled: bool`, `aria_label: String`, `start: Element`, `badge: u32` | Icon-only buttons need `aria_label`. `expand` fills the width. |
| `Fab` | `children`, `vertical: FabVertical`, `horizontal: FabHorizontal`, `edge: bool` | Pass it to `Body { fab: .. }` so it floats over the scroll area. |
| `FabButton` | `children`, `onclick`, `size: FabSize`, `activated: bool`, `close_icon: Element`, `href: String`, `disabled: bool` | |
| `FabList` | `children`, `activated: bool`, `side: FabListSide` | The mini buttons a `FabButton` reveals. |
| `FabContainer` | `main_button: Element`, `list_buttons: Element`, `vertical`, `horizontal`, `list_side` | `Fab` + `FabButton` + `FabList` with the open state managed for you. |
| `InfoButton` | `onclick`, `aria_label: String` | The small ⓘ. |
| `SheetButton` | `description: String` | An `InfoButton` that opens a bottom sheet with the text. |

- `ButtonStyle`: `Solid` (default, primary), `Outline` (secondary), `Clear`
  (text only; header buttons), `Neutral` (a surface color), `Danger`
  (destructive, text only).
- `ButtonSize`: `Sm`, `Md` (default), `Lg`.
- `FabVertical`: `Top`, `Center`, `Bottom`. `FabHorizontal`: `Start`, `Center`,
  `End`. `FabListSide`: `Top`, `Bottom`, `Start`, `End`. `FabSize`: `Normal`,
  `Small`.

---

## Forms

| Component | Props | Notes |
| --- | --- | --- |
| `Field` | `label: String` (required), `value: Signal<String>`, `placeholder`, `multiline: bool`, `rows: u32`, `maxlength: usize`, `minlength`, `min`/`max: isize`, `disabled: bool`, `debounce: u32`, `end: Element`, `oninput`, `onchange` | Also takes any `input` attribute, e.g. `r#type: "email"`, `autocomplete: "email"`. The label is the accessible name, so Playwright finds it with `getByLabel`. |
| `Select` | `value: Signal<String>`, `options: Vec<SelectOption>`, `disabled: bool`, `onchange: EventHandler<String>` | `SelectOption::from("Blue")` or `SelectOption::from(("blue", "Blue"))` for a value and a label. |
| `Toggle` | `checked: Signal<bool>`, `onchange: Callback<bool>`, `size: ToggleSize` | Usually the `end` of an `Item`. `ToggleSize`: `Sm` (default), `Md`. Role `switch`. |
| `Checkbox` | `checked: Signal<bool>`, `label: String` (required), `indeterminate`, `disabled`, `error: String`, `hint: String`, `label_placement: ControlLabelPlacement`, `onchange: Callback<bool>` | |
| `RadioGroup` | `value: Signal<String>`, `children`, `name`, `disabled`, `allow_empty_selection: bool`, `on_change: Callback<String>` | Contains `Radio`s. |
| `Radio` | `value: String` (required, non-empty), `label: String`, `disabled`, `placement: ControlLabelPlacement` | |
| `SegmentGroup` | `active: Signal<usize>`, `children`, `on_change: Callback<usize>`, `defer_active: bool` | A tablist; each button has role `tab`. |
| `SegmentButton` | `index: usize`, `children`, `disabled: bool` | |

`ControlLabelPlacement`: `Start` (default), `End`, `Fixed`, `Stacked`.

A segmented control whose selection belongs in the URL (a filter, a tab) keeps a
local mirror of the index and navigates on change — see `NotesToolbar` in
`src/components/notes/note_list.rs`.

---

## Content

| Component | Props | Notes |
| --- | --- | --- |
| `Card` | `children`, `title: String`, `right_slot: RightSlot`, `image: Element`, `inset: bool`, `selected: bool`, `onclick` | The default surface for a block of content, including empty and error states. `RightSlot::Text(..)` or `RightSlot::Element(rsx! { .. })`. A card inside a card or on a sheet gets a contrasting tint automatically. |
| `List` | `children`, `inset: bool`, `lines: ListLines` | `inset: true` for the rounded, grouped iOS-settings look. |
| `Item` | `label`, `description`, `overline`, `metadata: String`, `start`/`end: Element`, `kind: ItemKind`, `detail: ItemDetail`, `onclick`, `selected`, `disabled`, `children` | A row. Settings rows, navigation rows, and data rows are all `Item`. |
| `ItemDivider` | `children` | A section heading inside a `List`. |
| `SwipeItem` | `children` (an `Item`), `start_actions`/`end_actions: Element`, `behavior: SwipeBehavior`, `on_full_swipe`, `on_long_press`, `disabled` | Swipe to reveal actions. |
| `SwipeAction` | `side: SwipeSide`, `children`, `onclick`, `accent: bool`, `destructive: bool` | |
| `AccordionGroup` | `children`, `value: Signal<Vec<String>>`, `multiple: bool`, `on_change` | |
| `AccordionItem` | `value: String`, `label: String`, `description`, `header: Element`, `disabled`, `children` | |
| `Badge` | `children`, `color: StatusColor` | |
| `Chip` | `children`, `selected`, `disabled`, `start`/`end: Element`, `onclick` | |
| `Avatar` | `src`, `alt`, `fallback: String` (initials), `size: AvatarSize` | |
| `Progress` | `value: f64`, `max: f64` | Omit `value` for indeterminate. |
| `Line` | `orientation: LineOrientation`, `margins: bool` | A divider. |

- `ListLines`: `Full`, `Inset` (default), `None`.
- `ItemKind`: `Static` (default), `Button` (focusable, press state),
  `Link(String)` (an anchor). Inside a native app an anchor navigates the
  WebView itself; to leave the app for the system browser, use a `Button` row
  that calls the `external-url` plugin.
- `ItemDetail`: `Auto` (chevron when interactive), `Show`, `Hide`.
- `SwipeSide`: `Start`, `End`. `SwipeBehavior`: `Reveal` (default), `Activate`,
  `Dismiss`.
- `StatusColor`: `Neutral` (default), `Accent`, `Success`, `Warning`, `Danger`.
- `AvatarSize`: `Sm`, `Md`, `Lg`. `LineOrientation`: `Horizontal`, `Vertical`.

```rust
List { inset: true,
    Item { label: "Dark mode", end: rsx! { Toggle { checked: dark } } }
    Item { kind: ItemKind::Button, label: "Sign out", onclick: move |_| {} }
    Item { kind: ItemKind::Link("https://dioxuslabs.com".into()), label: "Dioxus docs" }
}
```

---

## Overlays

| Component | Props | Notes |
| --- | --- | --- |
| `Toast` | `open: Signal<bool>`, `message: String`, `color: StatusColor`, `position: ToastPosition`, `duration_ms: u64` (default 3000), `action: Element`, `on_dismiss` | The template mounts one in `AppOverlays`. Call `app_state.show_toast(..)` rather than rendering another. |
| `ConfirmModal` | `open: Signal<bool>`, `title: String`, `on_confirm` (**required**), `description: Element`, `confirm_text`, `cancel_text` | For "are you sure?". |
| `Modal` | `open: Signal<bool>`, `title: String`, `description: Element`, `actions: Element`, `children` | For anything else in a dialog. |
| `Sheet` | `is_open: Signal<bool>`, `children`, `placement: SheetPlacement`, `backdrop: SheetBackdrop`, `draggable: bool` (default `true`) | A bottom sheet, or a side drawer. |

- `ToastPosition`: `Top`, `Middle`, `Bottom` (default).
- `SheetPlacement`: `Bottom` (default), `Left(SideSheetType)`,
  `Right(SideSheetType)`.
- `SideSheetType`: `Overlay` (default), `Push`, `Reveal`, `Menu` (a persistent
  rail).
- `SheetBackdrop`: `Dismiss` (default; tap the scrim to close), `None` (the page
  behind stays usable).

**A sheet, a modal, or a route?** If the thing should survive a refresh, be
linkable, or be closed by Back, make it a `#[transition(cover)]` route (the
template's note editor is one). If it is a quick choice that belongs to the
screen that opened it, use `Sheet` or `Modal`. See
[navigation.md](navigation.md).

To keep Android Back closing an open `Sheet` at the root of history, see "Sheets
and system Back" in [navigation.md](navigation.md).

---

## Feedback

| Component | Props | Notes |
| --- | --- | --- |
| `Spinner` | `center: bool` | The loading state for a `use_resource` that has not resolved. |
| `Skeleton` | `shape: SkeletonShape` | Placeholder shapes for content that is on its way. `SkeletonShape`: `Text` (default), `Block`, `Avatar`, `Row`. |
| `Refresher` | `children`, `on_refresh: Callback<()>`, `refreshing: bool`, `can_refresh: bool`, `threshold: f64`, `disabled: bool` | Pull to refresh. See the notes list. |

---

## Text

g3-ui styles its own components' text. For a paragraph of your own inside a
`Card`, use its text classes rather than writing colors:

| Class | Use |
| --- | --- |
| `g3-message-text` | Base size for supporting copy. Combine with one below. |
| `g3-message-text-muted` | Secondary copy (`--color-text-secondary`). |
| `g3-message-text-subtle` | Captions, placeholders (`--color-label-secondary`). |
| `g3-message-text-success` / `-warning` / `-danger` | Status copy. |

```rust
Card { title: "No notes yet",
    p { class: "g3-message-text g3-message-text-muted", "Notes you write show up here." }
}
```

---

## Theme

A `Theme` is 17 color tokens. `AppWrapper` writes each as a CSS custom property,
so anything you style yourself should reference these rather than a literal:

| Field | Custom property | Is |
| --- | --- | --- |
| `focused` | `--color-focused` | The accent: selection, focus, links, primary buttons |
| `bg` / `bg_secondary` | `--color-bg` / `--color-bg-secondary` | The page, and a recessed ground |
| `card` / `card_inset` / `card_border` | `--color-card` / `--color-card-inset` / `--color-card-border` | Card surfaces |
| `surface` | `--color-surface` | Sheets, modals, toasts |
| `control` | `--color-control` | Inputs and unselected segments |
| `text` / `text_secondary` | `--color-text` / `--color-text-secondary` | Body copy |
| `label_primary` / `label_secondary` | `--color-label-primary` / `--color-label-secondary` | Row labels and detail |
| `success` / `warning` / `danger` | `--color-success` / `--color-warning` / `--color-danger` | Status |
| `shadow` | `--color-shadow` | Elevation |
| `color_scheme` | `color-scheme` | `"light"` or `"dark"` |

`Theme::default_light()` and `Theme::default_dark()` are the presets.
`.with_focused("#16a34a")` swaps the accent; struct-update syntax
(`Theme { card: "#fff".into(), ..Theme::default_light() }`) replaces anything
else. See [styling.md](styling.md).

## Modes

`ComponentMode::Ios` or `ComponentMode::Md`. The template drives it from the
user's saved preference through `AppState::set_appearance`, which also switches
g3-route-transitions' motion so the two never disagree. Never call
`g3_ui::set_mode` on its own.
