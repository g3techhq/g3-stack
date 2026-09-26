# Tests

Three layers, deliberately weighted toward the first.

## Rust unit tests

Next to the code, in `#[cfg(test)] mod tests`. Fast, no database, no browser.

```bash
just test           # web and server feature sets
cargo test          # web only, quicker while iterating
```

What is worth testing here — and what the template already does:

- **Transition intent** (`src/app.rs`). `#[transition(..)]` attributes are
  declarative, so what they *mean* is assertable without rendering anything.
  Adding a route later cannot silently change how an existing one animates.
- **Validation** (`src/db/note.rs`). Plain functions, compiled into both builds.
- **Auth decisions** (`src/auth/session.rs`). Which paths are unguarded, and
  whether a request is a navigation or a fetch.
- **Storage shapes** (`src/db/user.rs`). An enum constrained by the schema
  stores as exactly the string the schema expects.
- **Invariants nothing else would notice.** `src/health.rs` asserts the health
  router is merged after the layer stack; `src/auth/account.rs` reads
  `database/schema/` and fails if a table with an owner is left out of account
  deletion; `src/server_url.rs` pins what a mobile build accepts as its server.

Server-only tests need `--no-default-features --features server`; `just test`
runs both passes.

## Script tests

`scripts/rename.mjs` has its own tests, run with Node's built-in runner:

```bash
npm run test:scripts
```

## Playwright

End-to-end, against a real `dx serve` and a real database.

```bash
just db-up
just test-ui                      # both viewports
npm run test:ui:mobile            # Pixel 7 only
npm run test:ui:headed            # watch it happen
npm run test:ui:report            # open the last HTML report
```

Playwright starts `dx serve` itself. A cold Rust + WASM build takes a while, so
the first run has a long timeout; set `PLAYWRIGHT_BASE_URL` to reuse a server
you already have running:

```bash
PLAYWRIGHT_BASE_URL=http://127.0.0.1:8080 npm run test:ui
```

Two projects run: `mobile-chromium` (Pixel 7) and `desktop-chromium` (1440×900).
Both matter — g3-ui's shell moves its tab bar to a left rail at 48rem, so the
same tree renders differently and only the desktop pass exercises the rail.

`smoke.spec.mjs` is a few passes through the whole stack: a note's full life, a
clean hydration, history after sign-in, appearance, and account deletion. Keep it
small: browser tests are slow and flaky in proportion to how many there are. Add
one when the thing you are checking only exists in a browser — a transition, a
swipe, the responsive switch, hydration.

### Writing them

Query by role and accessible name (`getByRole("button", { name: "Save" })`), not
by CSS class. g3-ui components carry full ARIA, and class names are internal to
the library — a test bound to one breaks on an upgrade that changed nothing the
user can see.
