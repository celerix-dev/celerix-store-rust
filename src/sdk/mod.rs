/// Software Development Kit (SDK) for Celerix Store.
/// 
/// This module provides a high-level API for interacting with the store, including
/// automatic mode discovery and a remote TCP client.
pub mod client;
/// Automatic mode discovery and store initialization.
pub mod discovery;

pub use client::Client;
pub use discovery::new;
