pub mod engine;
pub mod sdk;
pub mod server;

use serde_json;
use thiserror::Error;
use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Error, Debug)]
pub enum Error {
    #[error("persona not found")]
    PersonaNotFound,
    #[error("app not found")]
    AppNotFound,
    #[error("key not found")]
    KeyNotFound,
    #[error("internal error: {0}")]
    Internal(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub const SYSTEM_PERSONA: &str = "_system";

#[async_trait]
pub trait KVReader: Send + Sync {
    async fn get(&self, persona_id: &str, app_id: &str, key: &str) -> Result<serde_json::Value>;
}

#[async_trait]
pub trait KVWriter: Send + Sync {
    async fn set(&self, persona_id: &str, app_id: &str, key: &str, value: serde_json::Value) -> Result<()>;
    async fn delete(&self, persona_id: &str, app_id: &str, key: &str) -> Result<()>;
}

#[async_trait]
pub trait AppEnumeration: Send + Sync {
    async fn get_personas(&self) -> Result<Vec<String>>;
    async fn get_apps(&self, persona_id: &str) -> Result<Vec<String>>;
}

#[async_trait]
pub trait BatchExporter: Send + Sync {
    async fn get_app_store(&self, persona_id: &str, app_id: &str) -> Result<HashMap<String, serde_json::Value>>;
    async fn dump_app(&self, app_id: &str) -> Result<HashMap<String, HashMap<String, serde_json::Value>>>;
}

#[async_trait]
pub trait GlobalSearcher: Send + Sync {
    async fn get_global(&self, app_id: &str, key: &str) -> Result<(serde_json::Value, String)>;
}

#[async_trait]
pub trait Orchestrator: Send + Sync {
    async fn move_key(&self, src_persona: &str, dst_persona: &str, app_id: &str, key: &str) -> Result<()>;
}

#[async_trait]
pub trait CelerixStore: KVReader + KVWriter + AppEnumeration + BatchExporter + GlobalSearcher + Orchestrator {
    fn app(&self, persona_id: &str, app_id: &str) -> Box<dyn AppScope + '_>;
}

#[async_trait]
pub trait AppScope: Send + Sync {
    async fn get(&self, key: &str) -> Result<serde_json::Value>;
    async fn set(&self, key: &str, value: serde_json::Value) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
    fn vault(&self, master_key: &[u8]) -> Box<dyn VaultScope + '_>;
}

#[async_trait]
pub trait VaultScope: Send + Sync {
    async fn get(&self, key: &str) -> Result<String>;
    async fn set(&self, key: &str, plaintext: &str) -> Result<()>;
}
