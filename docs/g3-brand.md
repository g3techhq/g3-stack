# Private g3tech integration

The public g3-stack template intentionally has no mandatory private dependencies.
g3tech-owned apps can add the private g3-brand library at a reviewed Git revision.
Its repository contains CI credential-helper/build templates and the full rollout
plan. Configure the read-only GitHub App before changing consumer dependencies.

Render the provided UI below the existing AppWrapper so it inherits the app theme
and mode. Use the same compatible g3-ui version; avoid two UI context versions.
Feed a validated catalog snapshot through the app's cached public read, preserving
the last good answer on refresh failure. Use a stable configured central HTTPS
endpoint without forwarding session cookies or user IDs. Refresh on section open
and foreground with the client's bounded interval.

Provide the stable app ID/channel and actual native installed metadata. Native
metadata comes from g3-native-plugins' app-info implementation once released; do
not replace it with a content-bundle version. The external-url plugin opens store
links. Keep legal/help/privacy routes available during a required update.

Apply g3-brand's app-schema.surql to the app database only when integrating its
backend receipt/consent helpers. Authenticate the subject with StateExtractor,
resolve the current published document set server-side, whitelist allowed consent
purposes, and record exact revisions plus the database timestamp. Acknowledgment,
terms acceptance and purpose consent remain distinct. Allow withdrawal without
forcing acceptance of a newer document.

Add the new private tables to the app's export, purge and retention inventory;
review any justified retained evidence. Verify users cannot read each other's
receipts. UI callbacks report success only after backend acknowledgment.

For destructive rights operations, persist a stable request ID and increasing
subject version in a transaction/outbox, then require the independent encrypted
recovery-ledger acknowledgment before processing. The host owns verification,
actual purge, processor propagation, session/cache clearing, secure export delivery,
and deadlines. Implement a release-specific idempotent replay command with apply
and verify modes; g3-platform's scratch restore requires it before handoff.

Configure SURREALDB_AUTH_LEVEL=database with a per-app runtime identity on a
shared server. Schema and administrative credentials stay separate. Production
privacy promises require verified provider retention, snapshots and restore
behavior, not only installing the library.
