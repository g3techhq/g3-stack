#!/usr/bin/env node
/**
 * Renames the template to your app.
 *
 * The template ships as `g3-app`, which appears in the crate name, the bundle
 * identifier, the container name, the database namespace, and the Dockerfile's
 * binary path. Renaming by hand means finding all of them; this finds them for
 * you.
 *
 *   node scripts/rename.mjs "Trail Notes" --identifier com.example.trailnotes
 *
 * The first argument is the human-readable name. Everything else is derived:
 * "Trail Notes" gives a `trail-notes` crate and binary, a `trail_notes` Rust
 * identifier, and `com.example.trailnotes` unless --identifier says otherwise.
 *
 * Pass --dry-run to see what would change without writing anything.
 */

import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));

// Build output, dependencies, and version control. Walking into `target`
// would take minutes and rewrite nothing that matters.
const SKIP_DIRS = new Set([
  "target",
  "node_modules",
  "dist",
  "data",
  ".git",
  "playwright-report",
  "test-results",
]);

// Text files only. A rename inside a binary would corrupt it.
const TEXT_EXTENSIONS = new Set([
  ".rs",
  ".toml",
  ".md",
  ".json",
  ".jsonc",
  ".yml",
  ".yaml",
  ".css",
  ".html",
  ".surql",
  ".mjs",
  ".cjs",
  ".js",
  ".template",
  ".sh",
]);

const NO_EXTENSION_FILES = new Set(["Dockerfile", "justfile", ".env.template", ".dockerignore"]);

// This script and its test mention the template's own names on purpose.
const SELF = new Set(["rename.mjs", "rename.test.mjs"]);

export function toKebab(name) {
  return name
    .trim()
    .replace(/['’]/g, "")
    .replace(/[^a-zA-Z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .toLowerCase();
}

/**
 * The ordered list of `[pattern, replacement]` pairs for a new name.
 *
 * Order matters: the specific spellings run before the bare `g3-app`, or they
 * never match. Every pattern is a regex so the bare one can refuse
 * `g3-app-shell`, which is g3-ui's container-query name rather than ours —
 * renaming it would silently break every `@container g3-app-shell` rule.
 */
export function replacementsFor(displayName, identifier) {
  const kebab = toKebab(displayName);
  const snake = kebab.replace(/-/g, "_");
  return [
    [/com\.example\.g3app/g, identifier || `com.example.${kebab.replace(/-/g, "")}`],
    [/G3 App/g, displayName],
    [/g3_app/g, snake],
    [/g3-app(?!-shell)/g, kebab],
  ];
}

export function applyReplacements(text, replacements) {
  return replacements.reduce((current, [pattern, to]) => current.replace(pattern, to), text);
}

function parseArgs(argv) {
  const positional = [];
  const flags = {};

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--dry-run") {
      flags.dryRun = true;
    } else if (arg === "--identifier") {
      flags.identifier = argv[i + 1];
      i += 1;
    } else if (arg.startsWith("--")) {
      throw new Error(`Unknown flag: ${arg}`);
    } else {
      positional.push(arg);
    }
  }

  return { positional, flags };
}

function walk(dir, out = []) {
  for (const entry of readdirSync(dir)) {
    if (SKIP_DIRS.has(entry)) continue;
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) {
      walk(path, out);
    } else {
      out.push(path);
    }
  }
  return out;
}

function isTextFile(path) {
  const name = path.split(/[/\\]/).pop().toLowerCase();
  if (SELF.has(name)) return false;
  if (NO_EXTENSION_FILES.has(name)) return true;
  const dot = name.lastIndexOf(".");
  return dot !== -1 && TEXT_EXTENSIONS.has(name.slice(dot));
}

function main() {
  const { positional, flags } = parseArgs(process.argv.slice(2));
  const displayName = positional[0];

  if (!displayName) {
    console.error(
      'Usage: node scripts/rename.mjs "Your App Name" [--identifier com.you.app] [--dry-run]',
    );
    process.exit(1);
  }

  const kebab = toKebab(displayName);
  if (!kebab) {
    console.error(`"${displayName}" has no letters or digits to build a crate name from.`);
    process.exit(1);
  }
  if (!/^[a-z]/.test(kebab)) {
    console.error(`"${kebab}" must start with a letter to be a valid crate name.`);
    process.exit(1);
  }
  if (kebab === "g3-app") {
    console.error("That is already the template's name. Nothing to do.");
    process.exit(1);
  }

  const replacements = replacementsFor(displayName, flags.identifier);
  let changedCount = 0;

  for (const path of walk(ROOT).filter(isTextFile)) {
    const original = readFileSync(path, "utf8");
    const updated = applyReplacements(original, replacements);

    if (updated !== original) {
      changedCount += 1;
      console.log(`${flags.dryRun ? "would update" : "updated"}  ${relative(ROOT, path)}`);
      if (!flags.dryRun) writeFileSync(path, updated);
    }
  }

  console.log(`\n${changedCount} file(s) ${flags.dryRun ? "would change" : "changed"}.`);
  console.log(`  crate/binary : ${kebab}`);
  console.log(`  display name : ${displayName}`);
  console.log(`  bundle id    : ${replacements[0][1]}`);

  if (!flags.dryRun) {
    console.log("\nNext:");
    console.log("  just check     refreshes Cargo.lock with the new name");
    console.log("  Replace assets/logo.svg, and review LICENSE and the URLs in README.md.");
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
