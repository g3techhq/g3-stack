---
name: verify-in-browser
description: Check a UI change in the running g3 stack app — start or reuse the dev server, sign in as a guest at a phone viewport, exercise the flow, and read the server log and browser console for errors. Use after changing a screen, route, transition, or server function the UI calls.
---

# Verify in the browser

Type-checking proves it compiles. This proves it works.

## 1. Database and server

1. The database must be running: `just db-up`. If the server log later says it
   cannot connect, check `SURREALDB_HOST` and `SURREALDB_PORT` in `.env`.
2. Reuse a dev server already answering on `http://127.0.0.1:8080`. Otherwise
   start the `web` configuration from `.claude/launch.json`, or run
   `dx serve --web --addr 127.0.0.1 --port 8080 --open false --interactive false`.
   Stop only a server you started.
3. A cold build takes minutes. Wait for "Serving your app" / "Build completed"
   in the output before loading the page.

## 2. Walk the flow

1. Set the viewport to **390×844** first.
2. Load `/`. The splash routes to sign-in; press **Continue as guest**. Every
   guest is a fresh, empty account.
3. Exercise the change the way a user would: tap, type, save, go back.
4. Transitions take ~400ms. Wait before each screenshot or you capture a
   half-finished animation and misread it as a bug.
5. If the change affects layout, repeat the key screens at **1440×900**, where
   the tab bar becomes a left rail.
6. If it touches appearance, switch Settings to Material and dark mode and look
   again.

## 3. Read the evidence

- **Server output** (`dx serve`): any `[500]`, panic, or SurrealDB error.
  `[401]` on an `/api/` call means the endpoint is guarded and the page had no
  session.
- **Browser console**: `Error deserializing data` is a hydration mismatch —
  markup that differs between server and client, or a head element beside a
  `use_server_future`. `already borrowed` is a signal borrow held across an
  await. A 404 on an asset is a path problem.
- Stop `dx serve` before editing Rust. A failed hot-patch leaves a build
  that stays on "Loading" with `Failed to resolve module specifier "env"` in
  the console; the fix is in `docs/troubleshooting.md`.
- A tap that navigates nowhere, with a clean console, can be the browser
  window being hidden: view transitions wait for a paint that never comes.
  Bring the window forward before calling it a bug.

## 4. Report

State what you exercised, at which viewports, and what the logs showed. If you
could not verify something (no database, a native-only plugin), say so plainly
rather than implying it works.
