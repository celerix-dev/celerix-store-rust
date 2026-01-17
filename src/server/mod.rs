/// TCP server implementation for the Celerix Store daemon.
/// 
/// This module provides the [`Router`] which handles incoming TCP connections
/// and dispatches commands to the underlying store.
pub mod router;

pub use router::Router;
