# Celerix Store (Rust)

[![crates.io](https://img.shields.io/crates/v/celerix-store.svg)](https://crates.io/crates/celerix-store)

A lightweight, low-latency Key-Value (KV) data store designed for the Celerix suite of applications. This is a high-performance Rust implementation that is 1:1 compatible with the original Go version.

## Key Features

- **Dual Mode Operations**:
    - **Embedded**: Use as a local library with direct file-based persistence.
    - **Remote**: Connect to a `celerix-stored` instance over TCP.
- **Liquid Data Architecture**: Uses the `Persona -> App -> Key` hierarchy for structured data management.
- **Atomic Persistence**: High-integrity "write-then-rename" strategy for JSON storage.
- **Client-Side Encryption**: Built-in AES-256-GCM vault support for sensitive data.
- **Automatic Discovery**: SDK automatically switches modes based on environment variables.
- **Resilient Client**: TCP client with automatic reconnection and exponential backoff retries.

## Quick Start

### As a Service (Daemon)
Run the `celerix-stored` binary to start the TCP server:
```bash
# Defaults to port 7001 and ./data directory
cargo run --bin celerix-stored
```

### As a Library (SDK)
Add this to your `Cargo.toml`:
```toml
[dependencies]
celerix-store = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

### CLI Tool
Interact with the store via the command line:
```bash
cargo run --bin celerix -- set my-persona my-app my-key '"my-value"'
cargo run --bin celerix -- get my-persona my-app my-key
```

## Documentation
- [USAGE.md](USAGE.md): Detailed usage patterns, library examples, and Docker/Podman deployment.
