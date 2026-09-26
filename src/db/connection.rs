use dioxus::prelude::Result;
#[cfg(debug_assertions)]
use surrealdb::engine::any::connect;
use surrealdb::{
    Surreal,
    engine::remote::ws::{Client, Ws},
    opt::auth::Root,
};

// Compiles every `.surql` file under `database/schema` into the binary, so a
// debug build can apply the schema at startup without the files being on disk
// next to it. Expands to the `embedded_schema` module used at the bottom of
// this file.
#[cfg(debug_assertions)]
surrealkit::embed_schema!("database/schema");

fn env(key: &str) -> String {
    std::env::var(key)
        .unwrap_or_else(|_| panic!("{key} is not set. Copy .env.template to .env and fill it in."))
}

fn surrealdb_host() -> String {
    env("SURREALDB_HOST")
}

/// The `Ws` connector adds the scheme itself and rejects a host that already
/// carries one, so a `SURREALDB_HOST` written either way works.
fn ws_connector_host(host: &str) -> &str {
    host.strip_prefix("ws://").unwrap_or(host)
}

/// The `any` connector wants the opposite: a full endpoint URL.
fn any_connector_endpoint(host: &str) -> String {
    if host.starts_with("ws://") || host.starts_with("wss://") {
        host.to_string()
    } else {
        format!("ws://{host}")
    }
}

/// Opens the long-lived connection every server function shares through
/// `StateExtractor`. Called once, from `main`.
pub async fn init_db_connection() -> Result<Surreal<Client>> {
    let db = Surreal::<Client>::init();
    let host = surrealdb_host();

    db.connect::<Ws>(ws_connector_host(&host)).await?;
    db.signin(Root {
        username: env("SURREALDB_USER"),
        password: env("SURREALDB_PASSWORD"),
    })
    .await?;
    db.use_ns(env("SURREALDB_NAMESPACE"))
        .use_db(env("SURREALDB_NAME"))
        .await?;

    Ok(db)
}

/// Applies `database/schema` to the running database.
///
/// Debug builds only, and called from `main` on every boot: editing a
/// `.surql` file and restarting `dx serve` is the whole local schema
/// workflow. Production goes through `surrealkit rollout` instead, so that a
/// destructive change is something a person approved rather than something a
/// deploy did on its way past. See database/README.md.
#[cfg(debug_assertions)]
pub async fn sync_dev_schema() -> anyhow::Result<()> {
    let db = connect(any_connector_endpoint(&surrealdb_host())).await?;
    db.signin(Root {
        username: env("SURREALDB_USER"),
        password: env("SURREALDB_PASSWORD"),
    })
    .await?;
    db.use_ns(env("SURREALDB_NAMESPACE"))
        .use_db(env("SURREALDB_NAME"))
        .await?;

    embedded_schema::sync(&db).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_connectors_accept_a_host_written_either_way() {
        assert_eq!(ws_connector_host("ws://localhost:8000"), "localhost:8000");
        assert_eq!(ws_connector_host("localhost:8000"), "localhost:8000");
        assert_eq!(
            any_connector_endpoint("localhost:8000"),
            "ws://localhost:8000"
        );
        assert_eq!(
            any_connector_endpoint("wss://db.example.com"),
            "wss://db.example.com"
        );
    }
}
