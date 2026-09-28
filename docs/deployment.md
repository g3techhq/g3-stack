# Deployment

The template ships a container build for the fullstack web server. Android and
iOS are `dx bundle`, then the usual store process.

`compose.yml` is development only — it publishes a root-credentialed SurrealDB
on loopback and has no app service.

---

## Environment

The server reads these at startup. Debug builds also load `.env`; release builds
read only the real environment, so a container needs them supplied.

| Variable | Notes |
| --- | --- |
| `SURREALDB_HOST` | `ws://` or `wss://`. Use `wss://` for anything crossing a network you do not own. |
| `SURREALDB_USER` / `SURREALDB_PASSWORD` | Root credentials. |
| `SURREALDB_NAMESPACE` / `SURREALDB_NAME` | Separate databases per environment — never point staging at production. |
| `PORT` / `IP` | What the server listens on. The image sets `8080` and `0.0.0.0`. |

The web image needs no URL of its own: the client calls the origin that served
it, so one image works behind any hostname. `SERVER_URL` only matters for mobile
builds — see [mobile.md](mobile.md).

If an app adds Google or Apple sign-in, its public client identifier is embedded
in the web bundle. Preserve one environment-neutral image by using one provider
identifier configured to allow every deployed origin or return URL. Supply that
same identifier as a Docker build argument for WASM and as a runtime variable
for server-side token validation. See [authentication.md](authentication.md#oauth-configuration-is-build-time-and-runtime).

---

## Building

```bash
docker build -t my-app:latest .
```

The Dockerfile uses `cargo-chef`, so a dependency layer is cached and an
app-code change rebuilds only the app. The first build is slow — it compiles the
whole tree twice, once for the host server and once for WASM.

The image exposes 8080 and reads `PORT` and `IP`.

### Health

| Path | Checks |
| --- | --- |
| `/api/v1/health` | Liveness — the process can serve HTTP |
| `/api/v1/health/ready` | Readiness — adds a SurrealDB round trip |

They are separate so a container runtime can tell "the process is wedged" from
"a dependency is down" and restart only in the first case. Point restart
policies at liveness and load-balancer registration at readiness.

The Dockerfile's `HEALTHCHECK` uses readiness, and a test in `src/health.rs`
asserts that it does.

---

## Running

A minimal production compose file. Two things in it are not optional:

```yaml
services:
  db-init:
    image: busybox:1.37
    user: "0:0"
    command: ["chown", "-R", "65532:65532", "/data"]
    volumes:
      - ./data-prod:/data
    restart: "no"

  db:
    image: surrealdb/surrealdb:v3.2.4
    restart: always
    command:
      - start
      - "--user=${DB_USER:?set DB_USER}"
      - "--pass=${DB_PASSWORD:?set DB_PASSWORD}"
      - surrealkv:///data/surrealkv
    volumes:
      - ./data-prod:/data
    depends_on:
      db-init:
        condition: service_completed_successfully
    ports:
      # Loopback. Docker's published ports bypass ufw and iptables, so an
      # unqualified "8000:8000" puts a root-credentialed database on every
      # interface of the host regardless of what the firewall says.
      - "127.0.0.1:${DB_PORT:-8000}:8000"
    healthcheck:
      test: ["CMD", "/surreal", "isready", "--endpoint", "http://127.0.0.1:8000"]
      interval: 10s
      timeout: 5s
      retries: 5

  app:
    # Digest-pinned and failing loudly. A mutable tag means a restart six months
    # from now silently deploys whatever moved under it.
    image: ${APP_IMAGE:?set APP_IMAGE to a digest-pinned image}
    restart: always
    depends_on:
      db:
        condition: service_healthy
    environment:
      SURREALDB_HOST: ws://db:8000
      SURREALDB_USER: ${DB_USER:?set DB_USER}
      SURREALDB_PASSWORD: ${DB_PASSWORD:?set DB_PASSWORD}
      SURREALDB_NAMESPACE: my-app
      SURREALDB_NAME: production
    ports:
      - "127.0.0.1:8080:8080"
```

Put TLS in front of it — a reverse proxy terminating HTTPS and forwarding to
8080. The session cookie is only as private as the transport.

---

## Schema changes

Release builds do not sync the schema. That is deliberate: SurrealKit's diff can
generate a destructive change, and a deploy is the wrong moment to discover
which one.

For a new, empty database, applying the schema directly is fine:

```bash
SURREALDB_HOST=... SURREALDB_USER=... SURREALDB_PASSWORD=... \
SURREALDB_NAMESPACE=... SURREALDB_NAME=... \
  surrealkit sync --fail-fast
```

For a database with data in it, use rollouts:

```bash
surrealkit rollout baseline                    # record the current state, once
surrealkit rollout plan --name add-tags        # generate a plan from the diff
# read database/rollouts/add-tags — this is the review step, not a formality
surrealkit rollout start add-tags
surrealkit rollout complete add-tags
```

Generated rollout files are committed, so what ran in production is in version
control alongside the code that expects it.

**Deploy order.** Apply an additive schema change (a new nullable field, a new
table, a new index) *before* the code that uses it, and a destructive one (a
dropped field, a narrowed type) *after* the last code that referenced it. A
deploy where the two cross produces errors on live traffic.

---

## Backups

`surreal export` writes a SurrealQL dump of a namespace and database, and
`surreal import` restores one. Whatever you wrap them in, the parts that matter:

- **Run it on a schedule and off the app host.** A backup on the same disk as
  the database is not a backup.
- **Restore it somewhere, on a schedule.** An untested backup is an assumption.
- **Keep the schema with the data.** A dump restored against a mismatched schema
  fails in ways that are hard to unpick; `database/schema/` at the matching
  commit is the other half of a restore.

---

## Mobile

```bash
SERVER_URL=https://app.example.com dx bundle --platform android --release
SERVER_URL=https://app.example.com dx bundle --platform ios --release
```

A release mobile build refuses to start without an HTTPS `SERVER_URL`. Signing,
deep links, and store requirements are in [mobile.md](mobile.md).

The manual **Build Mobile Artifacts** workflow keeps these expensive native
builds out of pull requests and normal `main` pushes. It uploads Android APK/AAB
and unsigned iOS IPA artifacts for 14 days. Treat those as compile outputs;
store distribution still requires an Android upload keystore or Apple
distribution certificate and provisioning profile.
