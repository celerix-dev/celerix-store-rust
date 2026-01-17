/// Core storage engine implementations for Celerix Store.
/// 
/// This module contains the in-memory store, filesystem persistence, and security primitives.
pub mod memstore;
/// Filesystem persistence logic.
pub mod persistence;
/// Cryptographic utilities for client-side encryption.
pub mod vault;

pub use memstore::MemStore;
pub use persistence::Persistence;
