//! Celerix Store is a lightweight, low-latency Key-Value (KV) data store.
//! 
//! It is designed with a "Liquid Data" architecture using a `Persona -> App -> Key` hierarchy.
//! This Rust implementation provides 1:1 parity with the original Go version, including
//! atomic persistence and AES-256-GCM client-side encryption.
//!
//! ## Core Components
//! - [`engine`]: The storage backend (In-memory with persistence).
//! - [`sdk`]: Client libraries for both embedded and remote (TCP) modes.
//! - [`server`]: TCP daemon implementation.

pub mod engine;
pub mod sdk;
pub mod server;

use serde_json;
use thiserror::Error;
use async_trait::async_trait;
use std::collections::HashMap;

/// Errors returned by the Celerix Store.
#[derive(Error, Debug)]
pub enum Error {
    /// The requested persona does not exist.
    #[error("persona not found")]
    PersonaNotFound,
    /// The requested app does not exist within the persona.
    #[error("app not found")]
    AppNotFound,
    /// The requested key does not exist within the app.
    #[error("key not found")]
    KeyNotFound,
    /// An internal error occurred.
    #[error("internal error: {0}")]
    Internal(String),
    /// An I/O error occurred during persistence or network communication.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Error during JSON serialization or deserialization.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// A specialized Result type for Celerix Store operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Reserved ID for global/system-level data.
pub const SYSTEM_PERSONA: &str = "_system";

/// Defines basic read operations for the store.
#[async_trait]
pub trait KVReader: Send + Sync {
    /// Retrieves a value for a specific persona, app, and key.
    async fn get(&self, persona_id: &str, app_id: &str, key: &str) -> Result<serde_json::Value>;
}

/// Defines basic write and delete operations for the store.
#[async_trait]
pub trait KVWriter: Send + Sync {
    /// Stores a value for a specific persona, app, and key.
    async fn set(&self, persona_id: &str, app_id: &str, key: &str, value: serde_json::Value) -> Result<()>;
    /// Deletes a key from a specific persona and app.
    async fn delete(&self, persona_id: &str, app_id: &str, key: &str) -> Result<()>;
}

/// Allows discovering personas and apps within the store.
#[async_trait]
pub trait AppEnumeration: Send + Sync {
    /// Lists all available persona IDs.
    async fn get_personas(&self) -> Result<Vec<String>>;
    /// Lists all app IDs for a given persona.
    async fn get_apps(&self, persona_id: &str) -> Result<Vec<String>>;
}

/// Allows retrieving bulk data from the store.
#[async_trait]
pub trait BatchExporter: Send + Sync {
    /// Returns all key-value pairs for a specific app within a persona.
    async fn get_app_store(&self, persona_id: &str, app_id: &str) -> Result<HashMap<String, serde_json::Value>>;
    /// Returns data for a specific app across all personas.
    async fn dump_app(&self, app_id: &str) -> Result<HashMap<String, HashMap<String, serde_json::Value>>>;
}

/// Allows searching for keys across all personas.
#[async_trait]
pub trait GlobalSearcher: Send + Sync {
    /// Finds a key within an app by searching all personas. Returns the value and the persona ID where it was found.
    async fn get_global(&self, app_id: &str, key: &str) -> Result<(serde_json::Value, String)>;
}

/// Handles higher-level data operations like moving keys between personas.
#[async_trait]
pub trait Orchestrator: Send + Sync {
    /// Moves a key from one persona to another within the same app.
    async fn move_key(&self, src_persona: &str, dst_persona: &str, app_id: &str, key: &str) -> Result<()>;
}

/// The primary interface for interacting with the Celerix Store.
/// 
/// It combines all functional traits for a complete storage experience.
#[async_trait]
pub trait CelerixStore: KVReader + KVWriter + AppEnumeration + BatchExporter + GlobalSearcher + Orchestrator {
    /// Returns an [`AppScope`] that simplifies operations by pinning a persona and app.
    fn app(&self, persona_id: &str, app_id: &str) -> Box<dyn AppScope + '_>;
}

/// A simplified, scoped interface for a specific persona and app.
#[async_trait]
pub trait AppScope: Send + Sync {
    /// Retrieves a value from the scoped app.
    async fn get(&self, key: &str) -> Result<serde_json::Value>;
    /// Stores a value in the scoped app.
    async fn set(&self, key: &str, value: serde_json::Value) -> Result<()>;
    /// Deletes a key from the scoped app.
    async fn delete(&self, key: &str) -> Result<()>;
    /// Returns a [`VaultScope`] for client-side encrypted storage using the provided master key.
    fn vault(&self, master_key: &[u8]) -> Box<dyn VaultScope + '_>;
}

/// A scoped interface for performing client-side encryption.
#[async_trait]
pub trait VaultScope: Send + Sync {
    /// Retrieves and decrypts a value from the scoped app.
    async fn get(&self, key: &str) -> Result<String>;
    /// Encrypts and stores a plaintext string in the scoped app.
    async fn set(&self, key: &str, plaintext: &str) -> Result<()>;
}
