# Database

## Layout

| Directory | What it is |
| --- | --- |
| `schema/` | The **desired** schema, one file per table. Not a migration log. |
| `seed/` | Demo and reference data. Re-runnable. |
| `setup.surql` | SurrealKit's own bookkeeping tables. Applied once; you should not need to edit it. |
| `rollouts/` | Generated production migrations. Committed. |
| `snapshots/` | Schema snapshots SurrealKit writes during a rollout. |

`schema/` is declarative: SurrealKit diffs these files against the running
database and works out what to change. That is why every statement is
`IF NOT EXISTS` — the file describes the end state, not a step.

## Local development

```bash
just db-up      # SurrealDB in Docker, on the port in .env
just dev        # dx serve — applies database/schema on startup
just db-seed    # optional demo data
```

Debug builds sync the schema on every boot (see `sync_dev_schema` in
`src/db/connection.rs`), so the local workflow is: edit a `.surql` file, restart
`dx serve`, done.

The manual equivalent, if you want to apply it without restarting:

```bash
surrealkit sync --fail-fast
```

## Remote operator workflow

Copy `.env.surrealkit.template` to an ignored file named for the target
environment and replace its environment marker, service URL, and credentials:

```powershell
Copy-Item .env.surrealkit.template .env.surrealkit.staging
```

The scoped launcher passes every remaining argument directly to SurrealKit:

```powershell
.\scripts\db.ps1 staging status
.\scripts\db.ps1 staging sync --fail-fast
.\scripts\db.ps1 staging seed
.\scripts\db.ps1 staging rollout status
.\scripts\db.ps1 staging rollout baseline
.\scripts\db.ps1 staging rollout plan --name add-tags
.\scripts\db.ps1 staging rollout lint <rollout-id>
.\scripts\db.ps1 staging rollout start <rollout-id>
.\scripts\db.ps1 staging rollout complete <rollout-id>
.\scripts\db.ps1 staging rollout rollback <rollout-id>
```

It runs from the repository root and restores the caller's environment when
SurrealKit exits. The development `.env` is never changed, so `just dev` and
`dx serve` keep using the local database.

`compose.yml` uses `surrealkv:///data/surrealkv`, so data survives a restart.
Swap it for `memory` if you would rather each restart start clean; delete
`./data` to reset either way.

## Conventions

**`PERMISSIONS NONE` on every table.** Nothing connects to SurrealDB except the
server, which authenticates as root. Authorization is the `owner = $user` clause
in each query (see `src/db/note.rs`). Row-level database permissions would be a
second source of truth about the same question, and the two disagreeing is worse
than either alone.

**Index what you filter by.** Every query in `note.rs` filters on `owner`, so
`note_owner_idx` exists. Without it each one is a full table scan, which is
invisible at ten rows and fatal at a hundred thousand.

**Constrain enums in the schema.** `appearance_mode ON user TYPE "ios" | "md"`
matches the Rust enum's `#[surreal(value = ..)]` spellings, so the two cannot
drift into disagreeing about what a valid value is. Without
`#[surreal(untagged)]` on the enum, the value stores as `{ Ios: {} }` and the
first insert fails; `AppearanceMode` in `src/db/user.rs` has a test to copy.

**One file per table**, named after it, matching a module in `src/db/`.

**Anything with an `owner` is deleted with the account.** Add a
`DELETE <table> WHERE owner = $user;` line to `DELETE_ACCOUNT_QUERY` in
`src/auth/account.rs`. A test reads this directory and fails if one is missing.

## Seeds

Every statement uses a fixed record id and `UPSERT`, so `just db-seed` is safe
to run repeatedly. Files are applied in filename order — the numeric prefix is
what orders them, so a seed that references another's rows sorts after it.

Do not run seeds against production.

## Production

Release builds do not sync the schema. SurrealKit's diff can generate a
destructive change, and a deploy is the wrong moment to find out which one.

New, empty database — applying directly is fine:

```powershell
.\scripts\db.ps1 production sync --fail-fast
```

Existing database with data — use rollouts:

```powershell
.\scripts\db.ps1 production rollout baseline
.\scripts\db.ps1 production rollout plan --name add-tags
# review database/rollouts/add-tags before continuing
.\scripts\db.ps1 production rollout start <rollout-id>
.\scripts\db.ps1 production rollout complete <rollout-id>
```

The review step is the point of the whole mechanism. Read the plan.

See [../docs/deployment.md](../docs/deployment.md) for deploy ordering — additive
changes go before the code that uses them, destructive ones after the last code
that referenced them.
