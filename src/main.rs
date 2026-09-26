mod app;
mod auth;
mod components;
mod db;
#[cfg(feature = "server")]
mod health;
mod server_url;
mod state;

use app::App;

cfg_if::cfg_if! { if #[cfg(feature = "server")] {

use std::sync::Arc;

use axum::middleware::from_fn;
use axum_session::{SessionConfig, SessionLayer, SessionStore};
use axum_session_auth::{AuthConfig, AuthSessionLayer};
use dioxus::{
    fullstack::{FullstackContext, axum_core::extract::FromRef},
    server::axum::Extension,
};
use surrealdb::{Surreal, engine::remote::ws::Client};

use crate::auth::{SessionUser, SurrealSessionPool, auth_check};
use crate::db::init_db_connection;
#[cfg(debug_assertions)]
use crate::db::sync_dev_schema;

pub use crate::auth::StateExtractor;

/// Everything a server function needs that is not per-request. Reached
/// through `StateExtractor` rather than directly, so a server function's
/// signature names what it uses instead of unpacking the world.
#[derive(Clone, Debug)]
pub struct AppServerState {
    db: Arc<Surreal<Client>>,
}

impl FromRef<FullstackContext> for AppServerState {
    fn from_ref(state: &FullstackContext) -> Self {
        state
            .extension::<AppServerState>()
            .expect("AppServerState extension is installed in main")
    }
}

fn main() {
    // Release builds read the environment the deployment gives them; only
    // local development goes looking for a file.
    #[cfg(debug_assertions)]
    dotenv::dotenv().ok();

    dioxus::serve(|| async move {
        let db = Arc::new(
            init_db_connection()
                .await
                .expect("Failed to connect to SurrealDB. Is it running (`just db-up`), and is .env filled in?"),
        );

        // Debug builds keep the running database in step with
        // `database/schema` on every boot. Release builds deliberately do
        // not: a production schema change should be a reviewed rollout, not
        // a side effect of a deploy. See database/README.md.
        #[cfg(debug_assertions)]
        sync_dev_schema()
            .await
            .expect("Failed to synchronize the development database schema.");

        let app_state = AppServerState { db: Arc::clone(&db) };

        let session_config = SessionConfig::default().with_cookie_path("/");
        let auth_config = AuthConfig::<String>::default();
        let session_store = SessionStore::new(
            Some(SurrealSessionPool::new(Arc::clone(&db))),
            session_config,
        )
        .await
        .expect("Failed to create the session store.");

        // Layer order is bottom-up: a request passes through SessionLayer,
        // then AuthSessionLayer, then `auth_check`, then reaches a route. Each
        // layer here depends on the one listed below it having already run.
        let router = dioxus::server::router(App)
            .layer(Extension(app_state))
            .layer(from_fn(auth_check))
            .layer(
                AuthSessionLayer::<
                    SessionUser,
                    String,
                    SurrealSessionPool<Client>,
                    Arc<Surreal<Client>>,
                >::new(Some(Arc::clone(&db)))
                .with_config(auth_config),
            )
            .layer(SessionLayer::new(session_store))
            // Merged, not layered, and merged last: `Router::layer` only wraps
            // routes registered before it, so probes reaching the app this way
            // skip the session and auth layers entirely. A health check should
            // not allocate a session row or get redirected to sign-in.
            .merge(crate::health::health_router(Arc::clone(&db)));

        Ok(router)
    });
}

}}

#[cfg(not(feature = "server"))]
fn main() {
    // Before launch, and before anything can make a request: a native build
    // has no cookie jar of its own, so without this the session cookie from
    // signing in never comes back on the next call.
    #[cfg(feature = "mobile")]
    dioxus_cookie::init();

    server_url::configure();
    dioxus::launch(App);
}
