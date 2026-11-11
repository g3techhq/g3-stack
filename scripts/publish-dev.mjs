#!/usr/bin/env node
/**
 * Publishes a dev build: a debug-signed Android APK with the in-app updater
 * (`--features mobile,sideload`), to g3techhq/releases, where installed dev
 * builds find it and update themselves.
 *
 *   node scripts/publish-dev.mjs --version 0.1.1 [--notes @notes.md] [--dry-run]
 *   just publish-dev --version 0.1.1
 *
 * 1. Sets `version` in Cargo.toml when `--version` differs (Android's
 *    versionCode follows it, and must rise with every release).
 * 2. Builds the APK with `dx build`, for the server in `SERVER_URL` (see
 *    docs/mobile.md): a dev build that people install should name one.
 * 3. Runs `scripts/publish-local.mjs` in a checkout of g3techhq/releases
 *    beside this repository (`../releases`, or `--releases <dir>`), with the
 *    secret key from `~/.g3-release/<crate name>-updater.key` (`--key`).
 *
 * The release key's public half must already be in src/updater.rs; see
 * docs/deployment.md, "In-app updates". Commit the version change afterwards.
 */

import { execFileSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { ROOT } from "./rename.mjs";

/** A `key = "value"` line from Cargo.toml's `[package]` table. */
export function packageField(toml, key) {
  const table = toml.split(/^\[(?!package\])/m)[0];
  const match = table.match(new RegExp(`^${key}\\s*=\\s*"([^"]*)"`, "m"));
  if (!match) throw new Error(`Cargo.toml has no package ${key}`);
  return match[1];
}

/** Cargo.toml with the package version set to `version`. */
export function withPackageVersion(toml, version) {
  if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error(`${version} is not x.y.z`);
  const [table, ...rest] = toml.split(/^(?=\[(?!package\]))/m);
  const updated = table.replace(/^version\s*=\s*"[^"]*"/m, `version = "${version}"`);
  return [updated, ...rest].join("");
}

/** `owner/repo` from a GitHub remote URL, or `null`. */
export function githubRepo(url) {
  const match = url.trim().match(/github\.com[:/]([^/]+\/[^/]+?)(?:\.git)?$/);
  return match ? match[1] : null;
}

/** The app's display name: Dioxus.toml's `[web.app] title`. */
export function appTitle(dioxusToml, fallback) {
  const section = dioxusToml.split(/^\[web\.app\]$/m)[1];
  const match = section?.match(/^title\s*=\s*"([^"]*)"/m);
  return match ? match[1] : fallback;
}

/** The parsed command line. */
export function parseArgs(argv) {
  const options = { dryRun: false };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const value = () => {
      const next = argv[++index];
      if (next === undefined) throw new Error(`${arg} needs a value`);
      return next;
    };
    if (arg === "--version") options.version = value();
    else if (arg === "--notes") options.notes = value();
    else if (arg === "--key") options.key = value();
    else if (arg === "--releases") options.releases = value();
    else if (arg === "--dry-run") options.dryRun = true;
    else throw new Error(`Unknown option ${arg}`);
  }
  return options;
}

function findApk(dir) {
  if (!existsSync(dir)) return null;
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) {
      const found = findApk(path);
      if (found) return found;
    } else if (entry.endsWith(".apk")) return path;
  }
  return null;
}

function run(command, args, options = {}) {
  console.log(`> ${command} ${args.join(" ")}`);
  // No shell, so arguments arrive as they are; `dx`, `git` and `node` are
  // executables Node finds on PATH on every platform.
  execFileSync(command, args, { stdio: "inherit", ...options });
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const cargoPath = join(ROOT, "Cargo.toml");
  let cargo = readFileSync(cargoPath, "utf8");
  const app = packageField(cargo, "name");
  const version = options.version ?? packageField(cargo, "version");

  // The placeholder key's own comment line, which a real key replaces.
  if (
    readFileSync(join(ROOT, "src/updater.rs"), "utf8").includes("untrusted comment: PLACEHOLDER")
  ) {
    throw new Error(
      'src/updater.rs still has the placeholder release key; see docs/deployment.md, "In-app updates".',
    );
  }
  const releases = resolve(ROOT, options.releases ?? "../releases");
  if (!existsSync(join(releases, "scripts/publish-local.mjs"))) {
    throw new Error(`No g3techhq/releases checkout at ${releases} (pass --releases <dir>).`);
  }
  const key = options.key ?? join(homedir(), ".g3-release", `${app}-updater.key`);
  if (!existsSync(key)) throw new Error(`No release key at ${key}.`);
  if (!process.env.SERVER_URL) {
    console.warn("SERVER_URL is not set: the build will look for a dev server on this network.");
  }

  if (version !== packageField(cargo, "version")) {
    cargo = withPackageVersion(cargo, version);
    // Also on a dry run: the APK it builds has to carry the new version.
    writeFileSync(cargoPath, cargo);
    console.log(`Cargo.toml version ${version}`);
  }

  run(
    "dx",
    [
      "build",
      "--platform",
      "android",
      "--target",
      "aarch64-linux-android",
      "--no-default-features",
      "--fullstack",
      "false",
      "--features",
      "mobile,sideload",
    ],
    { cwd: ROOT },
  );
  const built = findApk(join(ROOT, "target/dx", app, "debug/android"));
  if (!built) throw new Error("dx build finished but no APK was found under target/dx.");
  mkdirSync(join(ROOT, "dist"), { recursive: true });
  // Release asset names may use only letters, digits, `.`, `_` and `-`.
  const apk = join(ROOT, "dist", `${app}-${version}.apk`);
  copyFileSync(built, apk);

  const git = (args, cwd = ROOT) => execFileSync("git", args, { cwd, encoding: "utf8" }).trim();
  const repo = githubRepo(git(["remote", "get-url", "origin"]));
  const sha = git(["rev-parse", "--short", "HEAD"]);
  const name = appTitle(readFileSync(join(ROOT, "Dioxus.toml"), "utf8"), app);

  run("git", ["-C", releases, "pull", "--ff-only"]);
  const args = [
    "scripts/publish-local.mjs",
    "--app",
    app,
    "--version",
    version,
    "--key",
    key,
    "--public-key",
    join(ROOT, "src/updater.rs"),
    "--file",
    `android=${apk}`,
    "--name",
    name,
    "--prerelease",
  ];
  if (repo) args.push("--source", `${repo}@${sha}`);
  if (options.notes) {
    const notes = options.notes.startsWith("@")
      ? `@${resolve(options.notes.slice(1))}`
      : options.notes;
    args.push("--notes", notes);
  }
  if (options.dryRun) args.push("--dry-run");
  run("node", args, { cwd: releases });
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main();
  } catch (error) {
    console.error(error.message);
    process.exit(1);
  }
}
