use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use crate::{CelerixStore, Result};
use log::{info, error};
use tokio::sync::Semaphore;

pub struct Router {
    store: Arc<dyn CelerixStore>,
    semaphore: Arc<Semaphore>,
}

impl Router {
    pub fn new(store: Arc<dyn CelerixStore>) -> Self {
        Self { 
            store,
            semaphore: Arc::new(Semaphore::new(100)),
        }
    }

    pub async fn listen(&self, port: &str) -> Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        info!("Celerix Store listening on port {}", port);

        loop {
            let (socket, _) = listener.accept().await?;
            let store = self.store.clone();
            let sem = self.semaphore.clone();

            tokio::spawn(async move {
                let _permit = match sem.try_acquire() {
                    Ok(p) => p,
                    Err(_) => {
                        error!("Server busy: too many concurrent connections. Rejecting...");
                        // Ensure it's closed
                        let mut socket = socket;
                        let _ = socket.shutdown().await;
                        return;
                    }
                };
                
                if let Err(e) = handle_connection(socket, store).await {
                    error!("Connection error: {}", e);
                }
            });
        }
    }
}

pub async fn handle_connection(mut socket: TcpStream, store: Arc<dyn CelerixStore>) -> Result<()> {
    let (reader, mut writer) = socket.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break;
        }

        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let command = parts[0].to_uppercase();
        let response = match command.as_str() {
            "GET" => {
                if parts.len() < 4 {
                    "ERR missing arguments".to_string()
                } else {
                    match store.get(parts[1], parts[2], parts[3]).await {
                        Ok(val) => format!("OK {}", serde_json::to_string(&val)?),
                        Err(e) => format!("ERR {}", e.to_string().to_lowercase()),
                    }
                }
            }
            "SET" => {
                if parts.len() < 5 {
                    "ERR missing arguments".to_string()
                } else {
                    let val_str = parts[4..].join(" ");
                    match serde_json::from_str(&val_str) {
                        Ok(val) => match store.set(parts[1], parts[2], parts[3], val).await {
                            Ok(_) => "OK".to_string(),
                            Err(e) => format!("ERR {}", e.to_string().to_lowercase()),
                        },
                        Err(_) => "ERR invalid json value".to_string(),
                    }
                }
            }
            "DEL" => {
                if parts.len() < 4 {
                    "ERR missing arguments".to_string()
                } else {
                    match store.delete(parts[1], parts[2], parts[3]).await {
                        Ok(_) => "OK".to_string(),
                        Err(e) => format!("ERR {}", e.to_string().to_lowercase()),
                    }
                }
            }
            "LIST_PERSONAS" => {
                match store.get_personas().await {
                    Ok(list) => format!("OK {}", serde_json::to_string(&list)?),
                    Err(e) => format!("ERR {}", e.to_string().to_lowercase()),
                }
            }
            "LIST_APPS" => {
                if parts.len() < 2 {
                    "ERR missing arguments".to_string()
                } else {
                    match store.get_apps(parts[1]).await {
                        Ok(list) => format!("OK {}", serde_json::to_string(&list)?),
                        Err(e) => format!("ERR {}", e.to_string().to_lowercase()),
                    }
                }
            }
            "DUMP" => {
                if parts.len() < 3 {
                    "ERR missing arguments".to_string()
                } else {
                    match store.get_app_store(parts[1], parts[2]).await {
                        Ok(data) => format!("OK {}", serde_json::to_string(&data)?),
                        Err(e) => format!("ERR {}", e.to_string().to_lowercase()),
                    }
                }
            }
            "DUMP_APP" => {
                if parts.len() < 2 {
                    "ERR missing arguments".to_string()
                } else {
                    match store.dump_app(parts[1]).await {
                        Ok(data) => format!("OK {}", serde_json::to_string(&data)?),
                        Err(e) => format!("ERR {}", e.to_string().to_lowercase()),
                    }
                }
            }
            "GET_GLOBAL" => {
                if parts.len() < 3 {
                    "ERR missing arguments".to_string()
                } else {
                    match store.get_global(parts[1], parts[2]).await {
                        Ok((val, persona)) => {
                            let out = serde_json::json!({
                                "persona": persona,
                                "value": val
                            });
                            format!("OK {}", serde_json::to_string(&out)?)
                        },
                        Err(e) => format!("ERR {}", e.to_string().to_lowercase()),
                    }
                }
            }
            "MOVE" => {
                if parts.len() < 5 {
                    "ERR missing arguments".to_string()
                } else {
                    match store.move_key(parts[1], parts[2], parts[3], parts[4]).await {
                        Ok(_) => "OK".to_string(),
                        Err(e) => format!("ERR {}", e.to_string().to_lowercase()),
                    }
                }
            }
            "PING" => "PONG".to_string(),
            "QUIT" => break,
            _ => "ERR unknown command".to_string(),
        };

        writer.write_all(format!("{}\n", response).as_bytes()).await?;
    }
    Ok(())
}
