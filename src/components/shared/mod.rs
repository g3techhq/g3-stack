//! Pieces more than one screen uses.

mod confirm;
// `error_code`, `friendly` and the rest are there for the screens that need them.
#[allow(
    dead_code,
    reason = "Shared definitions are used by selected platform, server or test builds."
)]
mod error_message;

mod resource_states;

pub use confirm::*;
pub use error_message::*;
pub use resource_states::*;
