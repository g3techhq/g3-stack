import assert from "node:assert/strict";
import { test } from "node:test";
import { applyReplacements, replacementsFor, toKebab } from "./rename.mjs";

const rename = (text, name = "Trail Notes", identifier) =>
  applyReplacements(text, replacementsFor(name, identifier));

test("derives a crate name from a display name", () => {
  assert.equal(toKebab("Trail Notes"), "trail-notes");
  assert.equal(toKebab("  Bob's  App! "), "bobs-app");
});

test("renames every spelling the template uses", () => {
  assert.equal(rename('name = "g3-app"'), 'name = "trail-notes"');
  assert.equal(rename("container_name: g3-app-db"), "container_name: trail-notes-db");
  assert.equal(rename("target/dx/g3-app/release"), "target/dx/trail-notes/release");
  assert.equal(rename("/wasm/g3_app_bg.wasm"), "/wasm/trail_notes_bg.wasm");
  assert.equal(rename('title = "G3 App"'), 'title = "Trail Notes"');
  assert.equal(rename('identifier = "com.example.g3app"'), 'identifier = "com.example.trailnotes"');
});

test("an explicit identifier wins", () => {
  assert.equal(rename("com.example.g3app", "Trail Notes", "dev.trail.notes"), "dev.trail.notes");
});

test("leaves g3-ui's container name alone", () => {
  // `g3-app-shell` belongs to g3-ui. Renaming it would break every responsive
  // rule in the app without a single error.
  const css = "@container g3-app-shell (width >= 48rem) { .x { gap: 1rem } }";
  assert.equal(rename(css), css);
});
