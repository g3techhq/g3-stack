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
///
/// Each step's error names the step and the variables behind it, and nothing
/// else: `main` prints only that outermost message, so a startup failure says
/// what to fix without the driver's text, which can echo a host or a query.
pub async fn init_db_connection() -> anyhow::Result<Surreal<Client>> {
    use anyhow::Context;

    let db = Surreal::<Client>::init();
    let host = surrealdb_host();

    db.connect::<Ws>(ws_connector_host(&host))
        .await
        .context("Could not reach SurrealDB at SURREALDB_HOST. Is it running (`just db-up`)?")?;
    authenticate(&db).await?;
    db.use_ns(env("SURREALDB_NAMESPACE"))
        .use_db(env("SURREALDB_NAME"))
        .await
        .context("Could not select SURREALDB_NAMESPACE and SURREALDB_NAME in SurrealDB.")?;

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
    authenticate(&db).await?;
    db.use_ns(env("SURREALDB_NAMESPACE"))
        .use_db(env("SURREALDB_NAME"))
        .await?;

    embedded_schema::sync(&db).await
}

// A shared server must not give app containers a root identity. Root remains
// available for existing local environments; production opts into database auth.
#[cfg(feature = "server")]
async fn authenticate<C: surrealdb::Connection>(db: &Surreal<C>) -> anyhow::Result<()> {
    use anyhow::Context;

    // `VarError` does not say which variable, so each read names its own.
    let read = |key: &'static str| {
        std::env::var(key).with_context(|| {
            format!("{key} is not set. Copy .env.template to .env and fill it in.")
        })
    };
    let username = read("SURREALDB_USER")?;
    let password = read("SURREALDB_PASSWORD")?;
    match std::env::var("SURREALDB_AUTH_LEVEL")
        .unwrap_or_else(|_| "root".into())
        .as_str()
    {
        "database" => {
            db.signin(surrealdb::opt::auth::Database {
                namespace: read("SURREALDB_NAMESPACE")?,
                database: read("SURREALDB_NAME")?,
                username,
                password,
            })
            .await
            .context("SurrealDB refused SURREALDB_USER and SURREALDB_PASSWORD as a database user (SURREALDB_AUTH_LEVEL=database) for SURREALDB_NAMESPACE and SURREALDB_NAME.")?;
        }
        "root" => {
            db.signin(Root { username, password })
                .await
                .context("SurrealDB refused SURREALDB_USER and SURREALDB_PASSWORD as a root user (SURREALDB_AUTH_LEVEL=root).")?;
        }
        _ => anyhow::bail!("SURREALDB_AUTH_LEVEL must be root or database"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_startup_error_prints_its_step_and_not_the_driver_text() {
        use anyhow::Context;
        // `main` panics with `{error}`: only the context written here may show.
        let failed: anyhow::Result<()> = Err(anyhow::anyhow!("driver text: ws://user:secret@db"))
            .context("Could not reach SurrealDB at SURREALDB_HOST.");
        let shown = format!("{}", failed.unwrap_err());
        assert_eq!(shown, "Could not reach SurrealDB at SURREALDB_HOST.");
    }

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
