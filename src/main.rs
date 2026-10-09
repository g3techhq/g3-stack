mod app;
mod auth;
mod components;
mod data_change;
mod db;
#[cfg(feature = "server")]
mod health;
mod server_url;
mod state;

use app::App;

cfg_if::cfg_if! { if #[cfg(feature = "server")] {

use std::sync::Arc;

use axum::middleware::from_fn_with_state;
use axum_session::{SessionConfig, SessionLayer, SessionStore};
use axum_session_auth::AuthConfig;
use dioxus::server::axum::Extension;
use g3_auth::{AuthGuard, AuthSessionLayer, SurrealSessionPool, require_session};
use surrealdb::engine::remote::ws::Client;

use crate::app::Route;
use crate::auth::AppUser;
use crate::db::init_db_connection;
#[cfg(debug_assertions)]
use crate::db::sync_dev_schema;

pub use crate::auth::StateExtractor;

fn main() {
    // Release builds read the environment the deployment gives them; only
    // local development goes looking for a file.
    #[cfg(debug_assertions)]
    dotenvy::dotenv().ok();

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

        // Secure outside debug builds: production is served over HTTPS, and a
        // session cookie must never ride along on a plain-HTTP request.
        let session_config = SessionConfig::default()
            .with_cookie_path("/")
            .with_secure(!cfg!(debug_assertions));
        let auth_config = AuthConfig::<String>::default();
        let session_store = SessionStore::new(
            Some(SurrealSessionPool::new(Arc::clone(&db))),
            session_config,
        )
        .await
        .expect("Failed to create the session store.");

        // Signed-out page loads go to the splash; `#[public]` on `Route` and on
        // server functions marks what they may reach. Panics here, at startup,
        // if the splash itself is not public, since the redirect would loop.
        let auth_guard = AuthGuard::for_routes(Route::Splash {});

        // Layer order is bottom-up: a request passes through the CDN guard,
        // SessionLayer, AuthSessionLayer, then the auth guard, then reaches a
        // route. Each layer here depends on the one listed below it having
        // already run. The database reaches `StateExtractor` through the
        // `Extension`.
        let router = dioxus::server::router(App)
            .layer(Extension(Arc::clone(&db)))
            .layer(from_fn_with_state(auth_guard, require_session::<AppUser, Client>))
            .layer(AuthSessionLayer::<AppUser, Client>::new(Some(Arc::clone(&db))).with_config(auth_config))
            .layer(SessionLayer::new(session_store))
            // Outside the session layer, so it sees (and drops) the cookie that
            // layer adds to a response marked `#[cache_shared(cdn = ..)]`, and
            // marks every other API response `private, no-cache`.
            .layer(g3_cache::cdn_cache_guard("/api"))
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
    g3_auth::init();

    server_url::configure();
    dioxus::launch(App);
}
