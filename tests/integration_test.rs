use celerix_store::engine::MemStore;
use celerix_store::sdk::Client;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct User {
    name: String,
    age: i32,
}

#[tokio::test]
async fn test_generic_helpers() {
    let store = Arc::new(MemStore::new(HashMap::new(), None));
    
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    
    let store_clone = store.clone();
    tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            let s = store_clone.clone();
            tokio::spawn(async move {
                let _ = celerix_store::server::router::handle_connection(socket, s).await;
            });
        }
    });
    
    let client = Client::connect(&addr.to_string()).await.unwrap();
    
    let user = User { name: "Alice".to_string(), age: 30 };
    client.set_generic("p1", "a1", "user1", &user).await.unwrap();
    
    let got_user: User = client.get_generic("p1", "a1", "user1").await.unwrap();
    assert_eq!(user, got_user);
}

#[tokio::test]
async fn test_full_protocol_integration() {
    let store = Arc::new(MemStore::new(HashMap::new(), None));
    
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    
    let store_clone = store.clone();
    tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            let s = store_clone.clone();
            tokio::spawn(async move {
                let _ = celerix_store::server::router::handle_connection(socket, s).await;
            });
        }
    });
    
    let stream = TcpStream::connect(addr).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    
    writer.write_all(b"PING\n").await.unwrap();
    let mut response = String::new();
    reader.read_line(&mut response).await.unwrap();
    assert_eq!(response.trim(), "PONG");
    
    writer.write_all(b"SET p1 app1 k1 \"v1\"\n").await.unwrap();
    response.clear();
    reader.read_line(&mut response).await.unwrap();
    assert_eq!(response.trim(), "OK");
    
    writer.write_all(b"GET p1 app1 k1\n").await.unwrap();
    response.clear();
    reader.read_line(&mut response).await.unwrap();
    assert_eq!(response.trim(), "OK \"v1\"");

    writer.write_all(b"GET_GLOBAL app1 k1\n").await.unwrap();
    response.clear();
    reader.read_line(&mut response).await.unwrap();
    assert!(response.trim().contains("p1"));
    assert!(response.trim().contains("v1"));
}
