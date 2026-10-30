use chrono::{DateTime, Utc};
use g3_auth::{RecordId, RecordIdKey};
use surrealdb_types::{Datetime, ToSql};

/// The plain, unquoted key of a record id, suitable for a URL path segment.
///
/// `RecordId` deliberately has no `Display`: SurrealQL needs quoting and
/// escaping (`club:⟨USA-PA-00520⟩`) that a URL segment does not want. Use
/// this for routes, and `ToSql::to_sql` for anything going back into a query.
pub fn record_key(id: &RecordId) -> String {
    match &id.key {
        RecordIdKey::String(key) => key.clone(),
        RecordIdKey::Number(key) => key.to_string(),
        RecordIdKey::Uuid(key) => key.to_string(),
        other => other.to_sql(),
    }
}

/// A past `Datetime` as a short relative label ("2h ago", "just now").
///
/// Computed on read rather than stored, so a row does not go stale sitting in
/// the database.
pub fn relative_time(datetime: &Datetime) -> String {
    let then: DateTime<Utc> = (*datetime).into_inner();
    let seconds = (Utc::now() - then).num_seconds().max(0);

    if seconds < 60 {
        "just now".to_string()
    } else if seconds < 3_600 {
        format!("{}m ago", seconds / 60)
    } else if seconds < 86_400 {
        format!("{}h ago", seconds / 3_600)
    } else if seconds < 604_800 {
        format!("{}d ago", seconds / 86_400)
    } else {
        format!("{}w ago", seconds / 604_800)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_the_last_minute_as_just_now() {
        assert_eq!(relative_time(&Datetime::from(Utc::now())), "just now");
    }

    #[test]
    fn formats_hours_and_days() {
        let three_hours = Datetime::from(Utc::now() - chrono::Duration::hours(3));
        let two_days = Datetime::from(Utc::now() - chrono::Duration::days(2));
        assert_eq!(relative_time(&three_hours), "3h ago");
        assert_eq!(relative_time(&two_days), "2d ago");
    }

    #[test]
    fn a_future_timestamp_does_not_produce_a_negative_label() {
        let ahead = Datetime::from(Utc::now() + chrono::Duration::hours(2));
        assert_eq!(relative_time(&ahead), "just now");
    }

    #[test]
    fn record_key_is_unquoted() {
        // The quoted SurrealQL form of this id is `note:⟨a-b⟩`; a route wants
        // the bare key.
        let id = RecordId::new("note", "a-b");
        assert_eq!(record_key(&id), "a-b");
    }

    #[test]
    fn record_ids_come_from_g3_auth() {
        fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    rust_files(&path, out);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    out.push(path);
                }
            }
        }
        let mut files = Vec::new();
        rust_files(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut files,
        );
        let mut offenders = Vec::new();
        for file in files {
            let text = std::fs::read_to_string(&file).unwrap();
            for (at, _) in text.match_indices("surrealdb_types::") {
                let path_end = text[at..]
                    .find(|c: char| c == ';' || c == '(' || c == '<' || c.is_whitespace())
                    .map_or(text.len(), |end| at + end);
                let group_end = text[at..].find(';').map_or(text.len(), |end| at + end);
                let end = if text[at..].starts_with("surrealdb_types::{") {
                    group_end
                } else {
                    path_end
                };
                let names = text[at..end].split(|c: char| !(c.is_alphanumeric() || c == '_'));
                if names
                    .into_iter()
                    .any(|name| name == "RecordId" || name == "RecordIdKey")
                {
                    offenders.push(file.display().to_string());
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "use g3_auth::RecordId instead in: {offenders:?}"
        );
    }
}
