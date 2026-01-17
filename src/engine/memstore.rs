use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use async_trait::async_trait;
use crate::{Result, Error, KVReader, KVWriter, AppEnumeration, BatchExporter, GlobalSearcher, Orchestrator, CelerixStore, AppScope, VaultScope};
use crate::engine::{Persistence, vault};

use std::sync::atomic::{AtomicUsize, Ordering};

type StoreData = HashMap<String, HashMap<String, HashMap<String, serde_json::Value>>>;

pub struct MemStore {
    data: RwLock<StoreData>,
    persistence: Option<Arc<Persistence>>,
    pending_tasks: Arc<AtomicUsize>,
}

impl MemStore {
    pub fn new(initial_data: StoreData, persistence: Option<Arc<Persistence>>) -> Self {
        Self {
            data: RwLock::new(initial_data),
            persistence,
            pending_tasks: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn wait(&self) {
        while self.pending_tasks.load(Ordering::SeqCst) > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    fn copy_persona_data(&self, persona_id: &str) -> Option<HashMap<String, HashMap<String, serde_json::Value>>> {
        let data = self.data.read().unwrap();
        data.get(persona_id).cloned()
    }

    async fn persist(&self, persona_id: String) {
        if let Some(p) = &self.persistence {
            if let Some(persona_data) = self.copy_persona_data(&persona_id) {
                let p = p.clone();
                let pending = self.pending_tasks.clone();
                pending.fetch_add(1, Ordering::SeqCst);
                tokio::task::spawn_blocking(move || {
                    if let Err(e) = p.save_persona(&persona_id, &persona_data) {
                        log::error!("Failed to persist persona {}: {}", persona_id, e);
                    }
                    pending.fetch_sub(1, Ordering::SeqCst);
                });
            }
        }
    }
}

#[async_trait]
impl KVReader for MemStore {
    async fn get(&self, persona_id: &str, app_id: &str, key: &str) -> Result<serde_json::Value> {
        let data = self.data.read().unwrap();
        data.get(persona_id)
            .ok_or(Error::PersonaNotFound)?
            .get(app_id)
            .ok_or(Error::AppNotFound)?
            .get(key)
            .cloned()
            .ok_or(Error::KeyNotFound)
    }
}

#[async_trait]
impl KVWriter for MemStore {
    async fn set(&self, persona_id: &str, app_id: &str, key: &str, value: serde_json::Value) -> Result<()> {
        {
            let mut data = self.data.write().unwrap();
            let persona = data.entry(persona_id.to_string()).or_default();
            let app = persona.entry(app_id.to_string()).or_default();
            app.insert(key.to_string(), value);
        }
        self.persist(persona_id.to_string()).await;
        Ok(())
    }

    async fn delete(&self, persona_id: &str, app_id: &str, key: &str) -> Result<()> {
        {
            let mut data = self.data.write().unwrap();
            if let Some(persona) = data.get_mut(persona_id) {
                if let Some(app) = persona.get_mut(app_id) {
                    app.remove(key);
                }
            }
        }
        self.persist(persona_id.to_string()).await;
        Ok(())
    }
}

#[async_trait]
impl AppEnumeration for MemStore {
    async fn get_personas(&self) -> Result<Vec<String>> {
        let data = self.data.read().unwrap();
        Ok(data.keys().cloned().collect())
    }

    async fn get_apps(&self, persona_id: &str) -> Result<Vec<String>> {
        let data = self.data.read().unwrap();
        Ok(data.get(persona_id)
            .map(|p| p.keys().cloned().collect())
            .unwrap_or_default())
    }
}

#[async_trait]
impl BatchExporter for MemStore {
    async fn get_app_store(&self, persona_id: &str, app_id: &str) -> Result<HashMap<String, serde_json::Value>> {
        let data = self.data.read().unwrap();
        data.get(persona_id)
            .ok_or(Error::PersonaNotFound)?
            .get(app_id)
            .cloned()
            .ok_or(Error::AppNotFound)
    }

    async fn dump_app(&self, app_id: &str) -> Result<HashMap<String, HashMap<String, serde_json::Value>>> {
        let data = self.data.read().unwrap();
        let mut result = HashMap::new();
        for (persona_id, apps) in data.iter() {
            if let Some(app_data) = apps.get(app_id) {
                result.insert(persona_id.clone(), app_data.clone());
            }
        }
        Ok(result)
    }
}

#[async_trait]
impl GlobalSearcher for MemStore {
    async fn get_global(&self, app_id: &str, key: &str) -> Result<(serde_json::Value, String)> {
        let data = self.data.read().unwrap();
        for (persona_id, apps) in data.iter() {
            if let Some(app_data) = apps.get(app_id) {
                if let Some(val) = app_data.get(key) {
                    return Ok((val.clone(), persona_id.clone()));
                }
            }
        }
        Err(Error::KeyNotFound)
    }
}

#[async_trait]
impl Orchestrator for MemStore {
    async fn move_key(&self, src_persona: &str, dst_persona: &str, app_id: &str, key: &str) -> Result<()> {
        let val = {
            let mut data = self.data.write().unwrap();
            let src_persona_data = data.get_mut(src_persona).ok_or(Error::PersonaNotFound)?;
            let src_app_data = src_persona_data.get_mut(app_id).ok_or(Error::AppNotFound)?;
            src_app_data.remove(key).ok_or(Error::KeyNotFound)?
        };

        self.set(dst_persona, app_id, key, val).await?;
        self.persist(src_persona.to_string()).await;
        
        Ok(())
    }
}

impl CelerixStore for MemStore {
    fn app(&self, persona_id: &str, app_id: &str) -> Box<dyn AppScope + '_> {
        Box::new(MemAppScope {
            store: self,
            persona_id: persona_id.to_string(),
            app_id: app_id.to_string(),
        })
    }
}

pub struct MemAppScope<'a> {
    store: &'a MemStore,
    persona_id: String,
    app_id: String,
}

#[async_trait]
impl<'a> AppScope for MemAppScope<'a> {
    async fn get(&self, key: &str) -> Result<serde_json::Value> {
        self.store.get(&self.persona_id, &self.app_id, key).await
    }

    async fn set(&self, key: &str, value: serde_json::Value) -> Result<()> {
        self.store.set(&self.persona_id, &self.app_id, key, value).await
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.store.delete(&self.persona_id, &self.app_id, key).await
    }

    fn vault(&self, master_key: &[u8]) -> Box<dyn VaultScope + '_> {
        Box::new(MemVaultScope {
            app: self,
            master_key: master_key.to_vec(),
        })
    }
}

pub struct MemVaultScope<'a> {
    app: &'a MemAppScope<'a>,
    master_key: Vec<u8>,
}

#[async_trait]
impl<'a> VaultScope for MemVaultScope<'a> {
    async fn get(&self, key: &str) -> Result<String> {
        let val = self.app.get(key).await?;
        let cipher_hex = val.as_str().ok_or_else(|| Error::Internal("Vault data is not a string".to_string()))?;
        vault::decrypt(cipher_hex, &self.master_key)
    }

    async fn set(&self, key: &str, plaintext: &str) -> Result<()> {
        let cipher_hex = vault::encrypt(plaintext, &self.master_key)?;
        self.app.set(key, serde_json::Value::String(cipher_hex)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_memstore_get_set() {
        let store = MemStore::new(HashMap::new(), None);
        store.set("p1", "app1", "k1", json!("v1")).await.unwrap();
        
        let val = store.get("p1", "app1", "k1").await.unwrap();
        assert_eq!(val, json!("v1"));
    }

    #[tokio::test]
    async fn test_memstore_delete() {
        let store = MemStore::new(HashMap::new(), None);
        store.set("p1", "app1", "k1", json!("v1")).await.unwrap();
        store.delete("p1", "app1", "k1").await.unwrap();
        
        let res = store.get("p1", "app1", "k1").await;
        assert!(matches!(res, Err(Error::KeyNotFound)));
    }

    #[tokio::test]
    async fn test_move_key() {
        let store = MemStore::new(HashMap::new(), None);
        store.set("p1", "app1", "k1", json!("v1")).await.unwrap();
        store.move_key("p1", "p2", "app1", "k1").await.unwrap();
        
        assert!(matches!(store.get("p1", "app1", "k1").await, Err(Error::KeyNotFound)));
        assert_eq!(store.get("p2", "app1", "k1").await.unwrap(), json!("v1"));
    }

    #[tokio::test]
    async fn test_app_scope_and_vault() {
        let store = MemStore::new(HashMap::new(), None);
        let master_key = b"thisis32byteslongsecretkey123456";

        let scope = store.app("p1", "a1");
        scope.set("secret", json!("hidden")).await.unwrap();

        let val = scope.get("secret").await.unwrap();
        assert_eq!(val, json!("hidden"));

        let v = scope.vault(master_key);
        v.set("password", "topsecret").await.unwrap();

        let pass = v.get("password").await.unwrap();
        assert_eq!(pass, "topsecret");

        // Check that it's encrypted in the underlying store
        let raw = scope.get("password").await.unwrap();
        assert_ne!(raw, json!("topsecret"));
        assert!(raw.is_string());
    }
}
