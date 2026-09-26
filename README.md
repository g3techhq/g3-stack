<h1 align="center">g3 stack</h1>

<p align="center">
  <strong>Ship one Rust app to the web, Android, and iOS — with native-feeling navigation, a real database, and auth already wired.</strong>
</p>

<p align="center">
  <a href="https://dioxuslabs.com"><img alt="Dioxus 0.7" src="https://img.shields.io/badge/Dioxus-0.7.9-e96020"></a>
  <a href="https://crates.io/crates/g3-ui"><img alt="g3-ui" src="https://img.shields.io/crates/v/g3-ui?label=g3-ui"></a>
  <a href="https://surrealdb.com"><img alt="SurrealDB 3" src="https://img.shields.io/badge/SurrealDB-3.2-ff00a0"></a>
  <a href="#license"><img alt="License: MIT or Apache-2.0" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue"></a>
</p>

<p align="center">
  <a href="#quick-start">Quick start</a> ·
  <a href="#whats-inside">What's inside</a> ·
  <a href="#the-g3-axioms">Axioms</a> ·
  <a href="#documentation">Docs</a> ·
  <a href="#building-with-an-ai-agent">AI agents</a>
</p>

---

A starter template for the g3 stack: [Dioxus](https://dioxuslabs.com) fullstack,
[SurrealDB](https://surrealdb.com), and three libraries that make a Dioxus app
feel native — [g3-ui](https://github.com/g3techhq/g3-ui) for components,
[g3-route-transitions](https://github.com/g3techhq/g3-route-transitions) for
navigation, and [g3-native-plugins](https://github.com/g3techhq/g3-native-plugins)
for the device.

Clone it and you have a running app with sign-in, a database with schema
management, a component library in iOS and Material styles, animated screen
transitions that respect system Back, account deletion for the app stores,
tests at three levels, CI, a container build, and instructions your coding
agent can follow. The part you write is your app.

## Quick start

You need [Rust](https://rustup.rs), [Docker](https://docker.com),
[Node 20+](https://nodejs.org), and two cargo tools:

```bash
cargo install cargo-binstall
cargo binstall dioxus-cli@0.7.9 just
```

Then:

```bash
git clone https://github.com/g3techhq/g3-stack my-app
cd my-app
node scripts/rename.mjs "My App"   # crate name, bundle id, database namespace
cp .env.template .env
just setup                         # npm install + git hooks
just db-up                         # SurrealDB in Docker
just dev                           # http://localhost:8080
```

Open <http://localhost:8080> and press **Continue as guest**. The first build
compiles the whole stack for two targets and takes a few minutes; after that,
`dx` hot-reloads.

> On GitHub, **Use this template** gives you a fresh repository without this
> one's history. Then clone yours and continue from `rename.mjs`.

## What's inside

| Layer | Library | In this template |
| --- | --- | --- |
| App framework | [Dioxus 0.7](https://dioxuslabs.com/learn/0.7/) | One `src/` for web (WASM), Android, and iOS. Components, signals, router, and server functions |
| Components | [g3-ui](https://github.com/g3techhq/g3-ui) | 30+ Ionic-style components with iOS and Material modes, dark mode, a desktop rail layout, and ARIA. Every screen is built from them |
| Navigation | [g3-route-transitions](https://github.com/g3techhq/g3-route-transitions) | Push, sheet, and fade transitions declared on the route enum, with Android Back and the iOS edge swipe wired to the same animation |
| Device APIs | [g3-native-plugins](https://github.com/g3techhq/g3-native-plugins) | Share sheet, clipboard, system browser, and system Back today; storage, auth, deep links, location, purchases, and media one feature flag away |
| Database | [SurrealDB 3](https://surrealdb.com) + [SurrealKit](https://crates.io/crates/surrealkit) | Schema as files, applied automatically in development and through reviewed rollouts in production |
| Server & auth | axum + `axum_session` | Session cookies stored in SurrealDB, a guard that protects every endpoint by default, guest sign-in, sign-out, and account deletion |
| Styling | g3-ui `Theme` + Tailwind v4 | Brand the app in one function. Tailwind for layout, nothing to install |
| Quality | clippy, rustfmt, Biome, typos, cargo-deny, Playwright, lefthook | `just quality` runs what CI runs |

### The example app

A small notes app, there to be read and then deleted:

- **Guest sign-in** that creates a real account and session
- **A list** with a segmented filter kept in the URL, swipe-to-pin, pull-to-refresh, and an empty state
- **A detail page** that pushes in and slides back
- **Create and edit sheets** that rise from the bottom and drop away when saved
- **Settings** switching iOS and Material, light and dark, exercising three native plugins, and deleting the account

Each file explains why it is shaped the way it is. When you are ready,
[remove the example](docs/adding-a-feature.md#removing-the-example).

## The g3 axioms

The template is opinionated. These are the opinions.

**1. Native feel is not optional.** A cross-platform app is judged against the
native apps next to it. So screens push and slide back, sheets rise and fall,
system Back animates the same way, and components look like iOS on iOS and
Material on Android. You get that by default and have to work to lose it.

**2. The UI library is the design system.** Screens are built from g3-ui
components and branded through one `Theme`. Custom CSS is the exception that
needs a reason — this template has one rule.

**3. Declare it once.** A screen's animation is an attribute on its route, not
code in a component. A table's shape is one `.surql` file. A server function is
one Rust function that is both the endpoint and the client call. When something
is declared in one place, it cannot disagree with itself.

**4. Safe by default.** Every endpoint requires a session unless you opt it out.
Every query is scoped to its owner. Every table's rows leave with the account.
The easy path is the secure one.

**5. Typed from the database to the pixel.** Rust types mirror the schema, cross
the wire unchanged, and feed components directly. A mismatch is a compile error,
not a bug report.

## How it fits together

```
Browser / Android / iOS
        │   server functions: typed Rust calls that compile to HTTP
        ▼
  axum ── SessionLayer → AuthSessionLayer → auth_check (guarded by default)
        │
        ▼
    SurrealDB
```

A server function is written once and called like any async function:

```rust
// src/db/note.rs — the body only exists in the server build.
#[post("/api/v1/create_note", crate::StateExtractor { db, session_user, .. }: crate::StateExtractor)]
pub async fn create_note(title: String, body: String) -> Result<Note> {
    let (title, body) = validate_note(&title, &body).map_err(dioxus::CapturedError::msg)?;
    db.create("note")
        .content(CreateNote { owner: session_user.record_id(), title, body, /* .. */ })
        .await?
        .ok_or_else(|| dioxus::CapturedError::msg("Failed to save the note."))
}

// src/components/notes/note_editor.rs — a client call with the same signature.
create_note(title, body).await
```

And a screen's motion is declared where its URL is:

```rust
#[transition(root, replace)]
#[route("/notes?:filter")]
Notes { filter: Option<NotesFilter> },

#[transition(pushed)]
#[route("/notes/:id")]
NoteDetail { id: String },

#[transition(cover)]
#[route("/notes/:id/edit")]
EditNote { id: String },
```

### Layout

```
src/
├── app.rs              ★ Routes, transitions, theme — read this first
├── state.rs            AppState: the user, appearance, shared overlays
├── main.rs             Server: database, session layers, router
├── server_url.rs       Where a mobile build sends its calls
├── auth/               Session guard, sign-in, sign-out, account deletion
├── db/                 One file per table: row types + server functions
│   └── note.rs         ★ The worked example
└── components/
    ├── shell/          Tab shell, page shell, back button, overlays
    ├── auth/           Splash, sign-in
    ├── notes/          ★ The example screens
    └── settings.rs     Appearance, native plugins, account

database/schema/        One .surql file per table
docs/                   Guides — start with architecture.md
tests/ui/               Playwright smoke tests
```

## Everyday commands

```bash
just dev            # web dev server, http://localhost:8080
just dev-android    # Android emulator or device
just dev-ios        # iOS simulator (macOS)

just db-up          # start SurrealDB
just db-seed        # load demo data
just db-reset       # stop SurrealDB and delete its data

just check          # type-check web, server, and mobile
just test           # Rust tests
just test-ui        # Playwright, phone and desktop viewports
just quality        # everything CI runs
```

`just` on its own lists everything.

## Documentation

| Guide | Covers |
| --- | --- |
| [Architecture](docs/architecture.md) | The three builds, a request end to end, state, rendering, hydration |
| [Adding a feature](docs/adding-a-feature.md) | Table → server functions → route → screen, and removing the example |
| [g3-ui reference](docs/g3-ui.md) | Every component and its props |
| [Navigation](docs/navigation.md) | Transitions, history, system Back |
| [Styling](docs/styling.md) | g3-ui first, theming, icons |
| [Authentication](docs/authentication.md) | Sessions, the guard, adding sign-in providers |
| [Native plugins](docs/native-plugins.md) | Device APIs and the cfg pattern |
| [Android and iOS](docs/mobile.md) | Running on devices, server URL, cookies, deep links, releasing |
| [Deployment](docs/deployment.md) | Container, environment, schema rollouts, backups |
| [Troubleshooting](docs/troubleshooting.md) | The problems people hit, and the fixes |
| [Database](database/README.md) | The schema workflow |
| [Tests](tests/README.md) | What is tested where |

## Building with an AI agent

The template is written to be worked on by coding agents as well as people.

- **[AGENTS.md](AGENTS.md)** holds the stack's rules — the ones a model trained
  on older Dioxus would get wrong — and a definition of done. Claude Code, Codex,
  Cursor, Copilot, Gemini CLI, and others read it automatically.
  [CLAUDE.md](CLAUDE.md) imports it.
- **[docs/g3-ui.md](docs/g3-ui.md)** is a complete component reference, so an
  agent builds with g3-ui instead of inventing markup.
- **Skills** in `.claude/skills/`: `add-feature` and `verify-in-browser`.
- **Tests that catch agent mistakes:** a route's animation, a table missing from
  account deletion, an unguarded endpoint, and a hydration mismatch all fail a
  test rather than a user.

A good first prompt: *"Read AGENTS.md, then add tags to notes using the
add-feature skill."*

## Developing against local g3 libraries

The template uses the published crates. To build against local checkouts,
create `.cargo/config.toml` (gitignored):

```toml
[patch.crates-io]
g3-ui = { path = "../g3-ui" }
g3-route-transitions = { path = "../g3-route-transitions" }
g3-native-plugins = { path = "../g3-native-plugins" }
```

## Community

- Questions and show-and-tell: the [Dioxus Discord](https://discord.gg/XgGxMSkvUM)
- Bugs in a g3 library: its GitHub issues —
  [g3-ui](https://github.com/g3techhq/g3-ui/issues),
  [g3-route-transitions](https://github.com/g3techhq/g3-route-transitions/issues),
  [g3-native-plugins](https://github.com/g3techhq/g3-native-plugins/issues)
- Bugs in the template: [g3-stack issues](https://github.com/g3techhq/g3-stack/issues)

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at
your option — the same terms as the g3 libraries. Code you build from the
template is yours to license however you like.
