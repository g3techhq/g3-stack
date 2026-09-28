//! What each kind of change makes stale in the client cache.
//!
//! A mutation reports what it changed with [`AppState::changed`](
//! crate::state::AppState::changed); only the cached reads that can see
//! that change refetch, instead of every read on every screen.
//!
//! Each list below is every cached read whose query touches a table the
//! change writes. It errs toward refetching: a read left off shows stale
//! data until its screen reopens or the app regains focus, while an extra
//! one only costs a request. When a server function starts reading another
//! table, or a mutation starts writing one, update the lists here in the
//! same change.

use g3_cache::invalidate_cached;

use crate::db::{get_note, list_notes};

/// What a mutation changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataChange {
    /// A note was created, edited, pinned or deleted (`create_note`,
    /// `update_note`, `set_note_pinned`, `delete_note`).
    Notes,
}

// A change that reaches nearly everything (an import, say) can call
// `g3_cache::invalidate_all_cached()` from its own variant.

/// Marks the cached reads `change` can affect stale, so mounted screens
/// refetch them while still showing what they had.
pub fn invalidate(change: DataChange) {
    match change {
        DataChange::Notes => {
            invalidate_cached(list_notes);
            invalidate_cached(get_note);
        }
    }
}
