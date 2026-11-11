import assert from "node:assert/strict";
import { test } from "node:test";
import {
  appTitle,
  githubRepo,
  packageField,
  parseArgs,
  withPackageVersion,
} from "./publish-dev.mjs";

const CARGO = `[package]
name = "trail-notes"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = { version = "1.0" }

[target.wasm32-unknown-unknown.dependencies.getrandom_v02]
package = "getrandom"
version = "0.2"
`;

test("reads the package's own name and version, not a dependency's", () => {
  assert.equal(packageField(CARGO, "name"), "trail-notes");
  assert.equal(packageField(CARGO, "version"), "0.1.0");
});

test("sets only the package version", () => {
  const updated = withPackageVersion(CARGO, "0.1.1");
  assert.equal(packageField(updated, "version"), "0.1.1");
  assert.match(updated, /^version = "0\.2"$/m);
  assert.equal(updated.replace('"0.1.1"', '"0.1.0"'), CARGO);
  assert.throws(() => withPackageVersion(CARGO, "v0.1.1"), /x\.y\.z/);
});

test("names the GitHub repository a remote points at", () => {
  assert.equal(githubRepo("https://github.com/g3techhq/trail-notes.git"), "g3techhq/trail-notes");
  assert.equal(githubRepo("git@github.com:g3techhq/trail-notes.git\n"), "g3techhq/trail-notes");
  assert.equal(githubRepo("https://example.com/x.git"), null);
});

test("takes the display name from Dioxus.toml", () => {
  assert.equal(appTitle('[web.app]\ntitle = "Trail Notes"\n', "trail-notes"), "Trail Notes");
  assert.equal(appTitle("[web]\n", "trail-notes"), "trail-notes");
});

test("parses the options and refuses unknown ones", () => {
  assert.deepEqual(parseArgs(["--version", "0.1.1", "--dry-run"]), {
    version: "0.1.1",
    dryRun: true,
  });
  assert.throws(() => parseArgs(["--version"]), /needs a value/);
  assert.throws(() => parseArgs(["--force"]), /Unknown option/);
});
