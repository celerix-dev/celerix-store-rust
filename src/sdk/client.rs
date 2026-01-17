use std::collections::HashMap;
use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use crate::{Result, Error, KVReader, KVWriter, AppEnumeration, BatchExporter, GlobalSearcher, Orchestrator, CelerixStore, AppScope, VaultScope};
use crate::engine::vault;
use tokio::sync::Mutex;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct Client {
    #[allow(dead_code)]
    addr: String,
    inner: Mutex<Option<ClientInner>>,
}

struct ClientInner {
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: tokio::net::tcp::OwnedWriteHalf,
}

impl Client {
    pub async fn connect(addr: &str) -> Result<Self> {
        let inner = Client::connect_inner(addr).await?;
        Ok(Self {
            addr: addr.to_string(),
            inner: Mutex::new(Some(inner)),
        })
    }

    async fn send_and_receive(&self, cmd: String) -> Result<String> {
        let mut inner_guard = self.inner.lock().await;
        
        // Retry logic
        for i in 0..3 {
            if inner_guard.is_none() {
                match Client::connect_inner(&self.addr).await {
                    Ok(inner) => *inner_guard = Some(inner),
                    Err(e) => {
                        if i == 2 { return Err(e); }
                        tokio::time::sleep(std::time::Duration::from_millis((i + 1) * 200)).await;
                        continue;
                    }
                }
            }

            let inner = inner_guard.as_mut().unwrap();
            if let Err(_) = inner.writer.write_all(format!("{}\n", cmd).as_bytes()).await {
                 *inner_guard = None;
                 continue;
            }

            let mut resp = String::new();
            match inner.reader.read_line(&mut resp).await {
                Ok(0) => {
                    *inner_guard = None;
                    continue;
                }
                Ok(_) => {
                    let resp = resp.trim();
                    if resp.starts_with("ERR") {
                        return Err(Error::Internal(resp[4..].to_string()));
                    }
                    return Ok(resp.to_string());
                }
                Err(_) => {
                    *inner_guard = None;
                    continue;
                }
            }
        }
        
        Err(Error::Internal("failed after 3 attempts".to_string()))
    }

    async fn connect_inner(addr: &str) -> Result<ClientInner> {
        let stream = TcpStream::connect(addr).await?;
        let (reader, writer) = stream.into_split();
        Ok(ClientInner {
            reader: BufReader::new(reader),
            writer,
        })
    }

    pub async fn get_generic<T: DeserializeOwned>(&self, persona_id: &str, app_id: &str, key: &str) -> Result<T> {
        let val = self.get(persona_id, app_id, key).await?;
        Ok(serde_json::from_value(val)?)
    }

    pub async fn set_generic<T: Serialize>(&self, persona_id: &str, app_id: &str, key: &str, value: T) -> Result<()> {
        let val = serde_json::to_value(value)?;
        self.set(persona_id, app_id, key, val).await
    }
}

#[async_trait]
impl KVReader for Client {
    async fn get(&self, persona_id: &str, app_id: &str, key: &str) -> Result<serde_json::Value> {
        let resp = self.send_and_receive(format!("GET {} {} {}", persona_id, app_id, key)).await?;
        let json_data = resp.strip_prefix("OK ").ok_or_else(|| Error::Internal("Invalid response".to_string()))?;
        Ok(serde_json::from_str(json_data)?)
    }
}

#[async_trait]
impl KVWriter for Client {
    async fn set(&self, persona_id: &str, app_id: &str, key: &str, value: serde_json::Value) -> Result<()> {
        let val_str = serde_json::to_string(&value)?;
        self.send_and_receive(format!("SET {} {} {} {}", persona_id, app_id, key, val_str)).await?;
        Ok(())
    }

    async fn delete(&self, persona_id: &str, app_id: &str, key: &str) -> Result<()> {
        self.send_and_receive(format!("DEL {} {} {}", persona_id, app_id, key)).await?;
        Ok(())
    }
}

#[async_trait]
impl AppEnumeration for Client {
    async fn get_personas(&self) -> Result<Vec<String>> {
        let resp = self.send_and_receive("LIST_PERSONAS".to_string()).await?;
        let json_data = resp.strip_prefix("OK ").ok_or_else(|| Error::Internal("Invalid response".to_string()))?;
        Ok(serde_json::from_str(json_data)?)
    }

    async fn get_apps(&self, persona_id: &str) -> Result<Vec<String>> {
        let resp = self.send_and_receive(format!("LIST_APPS {}", persona_id)).await?;
        let json_data = resp.strip_prefix("OK ").ok_or_else(|| Error::Internal("Invalid response".to_string()))?;
        Ok(serde_json::from_str(json_data)?)
    }
}

#[async_trait]
impl BatchExporter for Client {
    async fn get_app_store(&self, persona_id: &str, app_id: &str) -> Result<HashMap<String, serde_json::Value>> {
        let resp = self.send_and_receive(format!("DUMP {} {}", persona_id, app_id)).await?;
        let json_data = resp.strip_prefix("OK ").ok_or_else(|| Error::Internal("Invalid response".to_string()))?;
        Ok(serde_json::from_str(json_data)?)
    }

    async fn dump_app(&self, app_id: &str) -> Result<HashMap<String, HashMap<String, serde_json::Value>>> {
        let resp = self.send_and_receive(format!("DUMP_APP {}", app_id)).await?;
        let json_data = resp.strip_prefix("OK ").ok_or_else(|| Error::Internal("Invalid response".to_string()))?;
        Ok(serde_json::from_str(json_data)?)
    }
}

#[async_trait]
impl GlobalSearcher for Client {
    async fn get_global(&self, app_id: &str, key: &str) -> Result<(serde_json::Value, String)> {
        let resp = self.send_and_receive(format!("GET_GLOBAL {} {}", app_id, key)).await?;
        let json_data = resp.strip_prefix("OK ").ok_or_else(|| Error::Internal("Invalid response".to_string()))?;
        let out: serde_json::Value = serde_json::from_str(json_data)?;
        let persona = out["persona"].as_str().ok_or_else(|| Error::Internal("Missing persona".to_string()))?.to_string();
        let value = out["value"].clone();
        Ok((value, persona))
    }
}

#[async_trait]
impl Orchestrator for Client {
    async fn move_key(&self, src_persona: &str, dst_persona: &str, app_id: &str, key: &str) -> Result<()> {
        self.send_and_receive(format!("MOVE {} {} {} {}", src_persona, dst_persona, app_id, key)).await?;
        Ok(())
    }
}

impl CelerixStore for Client {
    fn app(&self, persona_id: &str, app_id: &str) -> Box<dyn AppScope + '_> {
        Box::new(RemoteAppScope {
            client: self,
            persona_id: persona_id.to_string(),
            app_id: app_id.to_string(),
        })
    }
}

pub struct RemoteAppScope<'a> {
    client: &'a Client,
    persona_id: String,
    app_id: String,
}

#[async_trait]
impl<'a> AppScope for RemoteAppScope<'a> {
    async fn get(&self, key: &str) -> Result<serde_json::Value> {
        self.client.get(&self.persona_id, &self.app_id, key).await
    }

    async fn set(&self, key: &str, value: serde_json::Value) -> Result<()> {
        self.client.set(&self.persona_id, &self.app_id, key, value).await
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.client.delete(&self.persona_id, &self.app_id, key).await
    }

    fn vault(&self, master_key: &[u8]) -> Box<dyn VaultScope + '_> {
        Box::new(RemoteVaultScope {
            app: self,
            master_key: master_key.to_vec(),
        })
    }
}

pub struct RemoteVaultScope<'a> {
    app: &'a RemoteAppScope<'a>,
    master_key: Vec<u8>,
}

#[async_trait]
impl<'a> VaultScope for RemoteVaultScope<'a> {
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
