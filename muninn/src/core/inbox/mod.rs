pub mod model;
pub mod postgres;
pub mod store;

pub use model::{InboxMessage, InboxStatus, NewInboxMessage};
pub use postgres::{mark_processed_tx, PgInboxStore};
pub use store::InboxStore;

#[cfg(test)]
mod tests;
