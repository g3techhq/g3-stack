import assert from "node:assert/strict";
import { test } from "node:test";
import { expectedTwins, toLiquid } from "./template.mjs";

test("maps each spelling of the template's name to its variable", () => {
  assert.equal(toLiquid('name = "g3-app"'), 'name = "{{ project-name }}"');
  assert.equal(toLiquid("/wasm/g3_app_bg.wasm"), "/wasm/{{ crate_name }}_bg.wasm");
  assert.equal(toLiquid('title = "G3 App"'), 'title = "{{ project-name | title_case }}"');
  assert.match(toLiquid('identifier = "com.example.g3app"'), /bundle_id/);
});

test("leaves g3-ui's own container name alone", () => {
  assert.equal(toLiquid("@container g3-app-shell (width >= 48rem) {}"), null);
});

test("has no twin for a file without the name", () => {
  assert.equal(toLiquid("fn main() {}"), null);
});

test("refuses a file that already speaks Liquid", () => {
  assert.throws(() => toLiquid('name = "g3-app" # {{ x }}', "Cargo.toml"), /Cargo\.toml/);
});

test("twins the files that make a project its own", () => {
  const twins = expectedTwins().map(([path]) => path);
  for (const path of ["Cargo.toml.liquid", "Dioxus.toml.liquid", "Dockerfile.liquid"]) {
    assert.ok(twins.includes(path), `${path} is missing`);
  }
  // Rendered as they are, the workflows' `${{ }}` would be read as Liquid.
  assert.ok(!twins.some((path) => path.startsWith(".github/")));
});
