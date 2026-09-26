//! Permanent account deletion.
//!
//! Not optional for an app that ships to a store: App Store Review Guideline
//! 5.1.1(v) and Google Play's user data policy both require that an app which
//! lets people create an account lets them delete it from inside the app.
//! It is in the template so the requirement is met from the first build.

use dioxus::prelude::*;

/// Every row that belongs to the account, then the account itself, in one
/// transaction — a half-finished purge leaves orphaned rows nobody can reach.
///
/// **Add a line here for every table you give an `owner` field.** A test
/// below reads `database/schema/` and fails if one is missing.
#[cfg(any(feature = "server", test))]
const DELETE_ACCOUNT_QUERY: &str = r#"
BEGIN TRANSACTION;
DELETE note WHERE owner = $user;
DELETE $user;
COMMIT TRANSACTION;
"#;

/// Deletes the signed-in account and everything it owns, then ends the
/// session.
#[post("/api/v1/delete_account", crate::StateExtractor { db, auth_session, session_user, .. }: crate::StateExtractor)]
pub async fn delete_account() -> Result<()> {
    db.query(DELETE_ACCOUNT_QUERY)
        .bind(("user", session_user.record_id()))
        .await?
        // `.await?` only reports a failure to run the request. A statement
        // that failed inside the transaction is in the response, and without
        // this the account would report as deleted while still existing.
        .check()?;

    auth_session.logout_user();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::DELETE_ACCOUNT_QUERY;

    #[test]
    fn every_table_with_an_owner_is_purged() {
        let schema_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("database/schema");

        for entry in std::fs::read_dir(&schema_dir).expect("database/schema exists") {
            let path = entry.expect("readable schema entry").path();
            let source = std::fs::read_to_string(&path).expect("readable schema file");

            for line in source.lines() {
                // `DEFINE FIELD IF NOT EXISTS owner ON <table> TYPE record<user>`
                let Some(rest) = line.split("FIELD IF NOT EXISTS owner ON ").nth(1) else {
                    continue;
                };
                let table = rest.split_whitespace().next().expect("a table name");

                assert!(
                    DELETE_ACCOUNT_QUERY.contains(&format!("DELETE {table} WHERE owner = $user")),
                    "{} gives `{table}` an owner, but deleting an account leaves its rows behind. \
                     Add it to DELETE_ACCOUNT_QUERY.",
                    path.display(),
                );
            }
        }
    }

    #[test]
    fn the_purge_is_all_or_nothing() {
        assert!(DELETE_ACCOUNT_QUERY.contains("BEGIN TRANSACTION;"));
        assert!(DELETE_ACCOUNT_QUERY.contains("COMMIT TRANSACTION;"));
        // The account row goes last, so a failure earlier cannot leave rows
        // pointing at a user that no longer exists.
        let user = DELETE_ACCOUNT_QUERY
            .find("DELETE $user")
            .expect("deletes the user");
        let note = DELETE_ACCOUNT_QUERY
            .find("DELETE note")
            .expect("deletes notes");
        assert!(note < user);
    }
}
