//! Events: one subscription to herdr per server, kept in a bounded ring
//! that `herdr_events_recent`, `_wait` and `_status` read (plan 05).
pub mod hub;
pub mod ring;

pub use hub::EventHub;
