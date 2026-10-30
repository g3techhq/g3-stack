# Multi-stage build for the fullstack web server. `cargo-chef` caches the
# dependency build so an app-code change does not recompile the whole tree.
FROM rust:1.95.0-trixie AS chef
RUN cargo install cargo-chef
# The web client is a wasm build; cooking its dependencies needs the target.
RUN rustup target add wasm32-unknown-unknown

# The Dioxus CLI produces the bundle.
RUN curl -L --proto '=https' --tlsv1.2 -sSf \
      https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall dioxus-cli@0.7.9 --root /.cargo -y --force
ENV PATH="/.cargo/bin:$PATH"

# `ring` (via rustls) needs a C toolchain to build.
RUN apt-get update \
    && apt-get install -y --no-install-recommends clang llvm curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# `getrandom` needs to be told it is running in a browser on the wasm target.
RUN printf '[target.wasm32-unknown-unknown]\nrustflags = ["--cfg", "getrandom_backend=\\"wasm_js\\""]\n' \
      > /usr/local/cargo/config.toml

WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Cook exactly what `dx bundle` builds, or it recompiles every dependency: the
# client for wasm32 with `web` under `wasm-release`, and the server with
# `server` under `server-release`. dx always names a target, so the server cook
# names the host triple too; without it the artifacts land in another directory.
RUN cargo chef cook --profile wasm-release --target wasm32-unknown-unknown \
      --no-default-features --features web \
      --recipe-path recipe.json --locked
RUN cargo chef cook --profile server-release --target "$(rustc -vV | sed -n 's/^host: //p')" \
      --no-default-features --features server \
      --recipe-path recipe.json --locked
COPY . .

# No SERVER_URL here. The web client calls the origin that served it, so one
# image works behind any hostname. Only mobile builds compile a URL in — see
# src/server_url.rs.
RUN dx bundle --platform web -r --locked --debug-symbols false

FROM debian:trixie-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/dx/g3-app/release/web/ /usr/local/app

ENV PORT=8080
ENV IP=0.0.0.0
EXPOSE 8080

# Readiness, not liveness: this one fails while the database is unreachable.
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8080/api/v1/health/ready || exit 1

WORKDIR /usr/local/app
# Dioxus 0.7.9 bundles the native executable as server.
RUN test -x /usr/local/app/server && test -s /usr/local/app/public/index.html
ENTRYPOINT ["/usr/local/app/server"]
