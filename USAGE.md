# Usage Guide

This guide covers how to use `celerix-store` as a Rust library and how to deploy it as a service using Docker or Podman.

## 1. Library Usage (SDK)

The `celerix-store` SDK provides a unified interface for both embedded (local) and remote (TCP) storage.

### Initialization

Add `celerix-store` to your `Cargo.toml`. You can use a local path for development or a Git URL for production/remote builds:

**Git Dependency (Recommended for remote builds):**
```toml
[dependencies]
celerix-store = { git = "https://github.com/your-org/celerix-store-rust.git", tag = "v0.1.0" }
```

**Local Path (For local development):**
```toml
[dependencies]
celerix-store = { path = "../celerix-store-rust" }
```

Use `sdk::new(data_dir)` to initialize the store. It automatically detects the `CELERIX_STORE_ADDR` environment variable to decide between Embedded and Remote modes.

```rust
use celerix_store::sdk;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Automatically switches to Remote mode if CELERIX_STORE_ADDR is set
    let store = sdk::new("./data").await?;
    
    // Standard CRUD
    store.set("persona1", "app1", "key1", serde_json::json!("value1")).await?;
    let val = store.get("persona1", "app1", "key1").await?;
    println!("Value: {}", val);

    Ok(())
}
```

### Scoped Access

Scopes allow you to "pin" a persona and application ID for cleaner code.

```rust
let app = store.app("my-persona", "my-app");
app.set("settings", serde_json::json!({"theme": "dark"})).await?;
let settings = app.get("settings").await?;
```

### Encrypted Vault

The `VaultScope` provides transparent client-side encryption using AES-256-GCM. Data is encrypted before being sent to the store or written to disk.

```rust
let master_key = b"thisis32byteslongsecretkey123456"; // Must be 32 bytes
let vault = app.vault(master_key);

// Encrypts and saves
vault.set("password", "top-secret-password").await?;

// Retrieves and decrypts
let pass = vault.get("password").await?;
println!("Decrypted: {}", pass);
```

### Generic Helpers (Remote Client only)

The `Client` implementation provides generic helpers for type-safe operations.

```rust
use celerix_store::sdk::Client;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct User { name: String }

let client = Client::connect("127.0.0.1:7001").await?;
client.set_generic("p1", "a1", "u1", User { name: "Alice".into() }).await?;
let user: User = client.get_generic("p1", "a1", "u1").await?;
```

---

## 2. Service Usage (Docker / Podman)

The `celerix-stored` daemon can be containerized for easy deployment.

### Configuration (Environment Variables)

| Variable | Description | Default |
|----------|-------------|---------|
| `CELERIX_PORT` | Port for the TCP server | `7001` |
| `CELERIX_DATA_DIR` | Directory for JSON persistence | `./data` |
| `CELERIX_DISABLE_TLS` | Must be set to `true` (TLS not yet supported in Rust version) | `true` |

### Example Dockerfile

```dockerfile
# Use the official Rust image to build the binary
FROM rust:1.75-slim as builder

WORKDIR /usr/src/celerix-store
COPY . .
RUN cargo build --release --bin celerix-stored

# Final stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/celerix-store/target/release/celerix-stored /usr/local/bin/celerix-stored

# Create data directory
RUN mkdir -p /app/data

EXPOSE 7001

ENV CELERIX_DATA_DIR=/app/data
ENV CELERIX_PORT=7001
ENV CELERIX_DISABLE_TLS=true

CMD ["celerix-stored"]
```

### Example docker-compose.yml

```yaml
version: '3.8'

services:
  celerix-store:
    build: .
    ports:
      - "7001:7001"
    volumes:
      - ./store-data:/app/data
    environment:
      - CELERIX_PORT=7001
      - CELERIX_DATA_DIR=/app/data
      - CELERIX_DISABLE_TLS=true
    restart: always
```

---

## 3. Deployment with Podman

Podman usage is identical to Docker. You can build and run using the same files:

```bash
podman build -t celerix-store .
podman run -d --name celerix-store -p 7001:7001 -v ./data:/app/data:Z celerix-store
```
