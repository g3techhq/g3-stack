# Styling

The short version: **g3-ui is the styling system.** Build screens out of its
components, brand the app through its `Theme`, and write CSS only for what is
left over. This template has one app-level CSS rule.

---

## The order to reach for things

1. **A g3-ui component or prop.** A page is `Header` + `Content`. A block of
   content is a `Card`. A row is an `Item` in a `List`. A vertical flow is a
   `Stack`. A full-width button is `Button { expand: ButtonExpand::Block }`.
   An empty or failed state is an `EmptyState`. The catalog, with props, is
   [g3-ui.md](g3-ui.md).
2. **`Text`**, for copy of your own: `Text { tone: TextTone::Secondary, ".." }`.
3. **A Tailwind layout utility** in `rsx!`, for arrangement g3-ui does not
   decide for you: `whitespace-pre-wrap` to keep a user's line breaks.
4. **A rule in `tailwind.css`**, only when the same styling repeats across
   screens or needs a container query.

What that buys you, at no cost: the iOS and Material looks, dark mode, the
desktop rail layout, safe-area insets, reduced motion, focus rings, and ARIA.
Every step down the list gives some of it up.

## What not to do

- **No color literals.** `color: #6b7280` ignores dark mode and every custom
  theme. Use a token (`var(--g3-color-text-secondary)`) or `color-mix()` over
  one: `color-mix(in srgb, var(--g3-color-accent) 16%, var(--g3-color-card))`.
- **No viewport media queries.** The g3-ui shell is a named container, and it is
  the shell that widens. Query it:

  ```css
  @container g3-app-shell (width >= 48rem) {
    .my-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  }
  ```

  An app embedded in a wide page then keeps its phone layout, and the desktop
  rail switch and your rule change at exactly the same width.
- **No overriding g3-ui's own classes** (`.g3-card-selected { .. }`). Its
  stylesheet loads after yours, class names are internal, and an upgrade can
  change either. If a component cannot look the way you need through its props
  and the theme, that is worth an issue on
  [g3-ui](https://github.com/g3techhq/g3-ui/issues) — the fix belongs in the
  library, where every app gets it.
- **No hand-built versions of components that exist.** A `div` styled like a
  card will not pick up the Material elevation or the desktop spacing.

Where g3-ui intends a value to be tuned, it exposes a custom property instead,
such as `--g3-sheet-max-height` for one sheet's height or
the rail width. Check g3-ui's stylesheet for the names. Setting those is fine.

---

## Theming

Everything about color is `app_theme` in `src/app.rs`:

```rust
pub fn app_theme(scheme: ColorScheme) -> Theme {
    let light = || Theme::default_light().with_accent("#2563eb");
    let dark = || Theme::default_dark().with_accent("#3b82f6");
    match scheme {
        ColorScheme::Auto => Theme::adaptive(light(), dark()),
        ColorScheme::Light => light(),
        ColorScheme::Dark => dark(),
    }
}
```

`Auto`, the default, is `Theme::adaptive`: every token becomes a CSS
`light-dark()` pair, so the page follows the system setting with no script, and
the server's HTML is already right. Change the two palettes and `Auto` follows.

`with_accent` changes the accent, which is most of what makes an app look like
itself: primary buttons, selected segments and tabs, toggles, focus rings, and
links all derive from it.

For a full palette, override any field with struct-update syntax (the fields
are listed in [g3-ui.md](g3-ui.md#theme)). A warm light theme, for example:

```rust
let light = || Theme {
    accent: "#a9530b".into(),
    bg: "#e8dcc8".into(),
    bg_secondary: "#dccdb4".into(),
    card: "#f5ede0".into(),
    surface: "#f0e6d5".into(),
    control: "#e2d4bd".into(),
    border: "#c9b391".into(),
    text: "#342a21".into(),
    text_secondary: "#6e5c4a".into(),
    text_tertiary: "#7a6653".into(),
    ..Theme::default_light()
};
```

Two things make a palette hold up:

- **Give every elevation its own visible step.** Page (`bg`), card (`card`),
  sheet (`surface`), and input (`control`) should each be distinguishable.
  White on near-white collapses them into one flat surface.
- **Tune dark separately.** Brighten the accent and the status colors against a
  dark ground rather than reusing the light values.

The theme is applied as custom properties on the shell element, so changing it
re-themes the running app in place — navigation and half-typed forms survive a
light/dark switch.

### Light, dark, iOS, Material

The template stores both preferences on the user's account (`appearance_mode`
and `color_scheme` on the `user` table) so they follow the account across
devices, and the server renders the first page in the saved theme. The settings
screen changes them through `AppState::set_appearance`, which also keeps
g3-route-transitions' motion in step with the component mode.

To follow the device instead, start from `g3_ui::init_auto_mode()` (iOS on
Apple devices, Material elsewhere) and read `prefers-color-scheme`.

---

## Icons

`dioxus-icons` ships the [Lucide](https://lucide.dev/icons/) set as components:

```rust
use dioxus_icons::lucide::{NotebookPen, Settings as SettingsIcon};

NotebookPen { size: 24 }
Pin { size: 18, color: "var(--g3-color-accent)" }
```

Import with an alias when a name collides with one of yours or with a Rust
prelude name. Colors, as always, come from tokens.

## App icon and favicon

`assets/logo.svg` is both. Replace it, and `Dioxus.toml`'s `[bundle] icon` picks
it up for Android and iOS builds.

## Tailwind

`dx` runs Tailwind v4 over `tailwind.css` on every build and writes
`assets/tailwind.css`, which is gitignored. There is nothing to install. The
`@source` line scans `src/` for class names used in `rsx!`.

Keep what Tailwind does to layout. g3-ui already decides color, type, radius,
and elevation, and a Tailwind color or shadow class fights it.
