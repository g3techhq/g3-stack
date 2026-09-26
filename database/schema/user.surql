-- The account. Guest sign-in creates one of these with nothing but a handle;
-- adding email or an OAuth provider later fills in the rest of the row
-- rather than creating a second one.
DEFINE TABLE IF NOT EXISTS user TYPE NORMAL SCHEMAFULL PERMISSIONS NONE;

DEFINE FIELD IF NOT EXISTS handle ON user TYPE string;
DEFINE FIELD IF NOT EXISTS display_name ON user TYPE string;
DEFINE FIELD IF NOT EXISTS email ON user TYPE option<string>;

-- Appearance preferences live on the account, not in browser storage, so a
-- signed-in user sees the same app on every device. `AppState` mirrors them
-- into signals at startup (see src/state.rs).
DEFINE FIELD IF NOT EXISTS appearance_mode ON user TYPE "ios" | "md" DEFAULT "ios";
DEFINE FIELD IF NOT EXISTS color_scheme ON user TYPE "light" | "dark" DEFAULT "light";

DEFINE FIELD IF NOT EXISTS created_at ON user TYPE datetime DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS user_handle_idx ON user FIELDS handle UNIQUE;
DEFINE INDEX IF NOT EXISTS user_email_idx ON user FIELDS email UNIQUE;
