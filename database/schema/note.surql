-- The example entity. Delete this file and src/db/note.rs when you start
-- building your own thing — they exist to show one feature wired end to end.
DEFINE TABLE IF NOT EXISTS note TYPE NORMAL SCHEMAFULL PERMISSIONS NONE;

DEFINE FIELD IF NOT EXISTS owner ON note TYPE record<user>;
DEFINE FIELD IF NOT EXISTS title ON note TYPE string;
DEFINE FIELD IF NOT EXISTS body ON note TYPE string DEFAULT "";
DEFINE FIELD IF NOT EXISTS pinned ON note TYPE bool DEFAULT false;
DEFINE FIELD IF NOT EXISTS created_at ON note TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at ON note TYPE datetime DEFAULT time::now();

-- Every query in src/db/note.rs filters by owner first; without this index
-- each one is a full table scan.
DEFINE INDEX IF NOT EXISTS note_owner_idx ON note FIELDS owner;
