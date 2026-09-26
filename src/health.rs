//! Container and load-balancer probes.

use std::sync::Arc;

use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use serde_json::json;
use surrealdb::{Surreal, engine::remote::ws::Client};

/// Liveness. The process answers as long as it can serve HTTP.
pub const HEALTH_PATH: &str = "/api/v1/health";

/// Readiness. Adds a SurrealDB round trip on top of liveness.
pub const READY_PATH: &str = "/api/v1/health/ready";

/// Deliberately does no work beyond proving the listener is up, so a container
/// runtime can tell "the process is wedged" apart from "a dependency is down"
/// and restart only in the first case.
async fn live() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

/// `Surreal::health` is a dedicated ping over the existing connection, cheap
/// enough to run on a short probe interval.
async fn ready(State(db): State<Arc<Surreal<Client>>>) -> impl IntoResponse {
    match db.health().await {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({ "status": "ok", "database": "ok" })),
        ),
        Err(error) => {
            eprintln!("Readiness check failed to reach SurrealDB: {error}");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "status": "degraded", "database": "unreachable" })),
            )
        }
    }
}

/// Merged into the app router *after* the session and auth layers, so probes
/// never allocate a session row and never get redirected to sign-in.
pub fn health_router(db: Arc<Surreal<Client>>) -> Router {
    Router::new()
        .route(HEALTH_PATH, get(live))
        .route(READY_PATH, get(ready))
        .with_state(db)
}

#[cfg(test)]
mod tests {
    use super::{HEALTH_PATH, READY_PATH};

    /// The health routes only stay unauthenticated and session-free while
    /// they are merged after the layer stack. `Router::layer` applies to the
    /// routes registered before it, so this ordering is load bearing and
    /// nothing else in the code would notice if it changed.
    #[test]
    fn the_health_router_is_merged_after_every_layer() {
        let source = include_str!("main.rs");

        let last_layer = source
            .rfind(".layer(")
            .expect("main builds the router with layers");
        let merge = source
            .find("health_router(")
            .expect("main merges the health router");

        assert!(
            last_layer < merge,
            "health_router must be merged after every .layer call so probes skip the session and auth layers"
        );
    }

    #[test]
    fn the_container_probes_readiness_not_liveness() {
        // A liveness probe here would keep a container in service while it
        // cannot reach the database.
        let dockerfile = include_str!("../dockerfile");
        assert!(dockerfile.contains("HEALTHCHECK"));
        assert!(dockerfile.contains(READY_PATH));
    }

    #[test]
    fn liveness_and_readiness_are_distinct_paths() {
        assert_ne!(HEALTH_PATH, READY_PATH);
        assert!(READY_PATH.starts_with(HEALTH_PATH));
    }

    #[test]
    fn compose_keeps_the_database_off_the_public_interface() {
        // Docker's published ports bypass the host firewall, so an
        // unqualified "8000:8000" would expose a root-credentialed database
        // on every interface of the machine.
        let compose = include_str!("../compose.yml");
        assert!(compose.contains("127.0.0.1:${SURREALDB_PORT"));
    }
}
