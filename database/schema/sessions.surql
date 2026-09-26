-- Backing table for `axum_session`. `SurrealSessionPool` (src/auth/session_store.rs)
-- reads and writes these rows; nothing in the app touches them directly.
--
-- SCHEMALESS on purpose: axum_session owns the shape, and pinning it here
-- would break on any change to that crate's serialized session payload.
DEFINE TABLE IF NOT EXISTS sessions TYPE NORMAL SCHEMALESS PERMISSIONS NONE;

DEFINE FIELD IF NOT EXISTS sessionid ON sessions TYPE string;
DEFINE FIELD IF NOT EXISTS sessionstore ON sessions TYPE string;
DEFINE FIELD IF NOT EXISTS sessionexpires ON sessions TYPE option<string>;

DEFINE INDEX IF NOT EXISTS sessions_sessionid_idx ON sessions FIELDS sessionid UNIQUE;
