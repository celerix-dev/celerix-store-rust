use std::env;
use std::sync::Arc;
use crate::{CelerixStore, Result};
use crate::engine::{MemStore, Persistence};
use crate::sdk::Client;

/// Initializes a [`CelerixStore`] based on the environment.
/// 
/// `new` automatically detects whether to connect to a remote server or 
/// initialize a local embedded engine:
/// 
/// 1. If `CELERIX_STORE_ADDR` environment variable is set, it attempts to 
///    connect to that address in **Remote Mode**.
/// 2. Otherwise, it initializes a [`MemStore`] with [`Persistence`] in the 
///    specified `data_dir` in **Embedded Mode**.
/// 
/// # Examples
/// 
/// ```no_run
/// use celerix_store::sdk;
/// 
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let store = sdk::new("./data").await?;
///     Ok(())
/// }
/// ```
pub async fn new(data_dir: &str) -> Result<Arc<dyn CelerixStore>> {
    if let Ok(addr) = env::var("CELERIX_STORE_ADDR") {
        if !addr.is_empty() {
            // Check for CELERIX_DISABLE_TLS - although we only support plain TCP for now,
            // we should warn or handle it if we want to be 100% parity-compliant.
            // Go version defaults to TLS unless CELERIX_DISABLE_TLS=true.
            // Our Rust version currently only supports plain TCP.
            if env::var("CELERIX_DISABLE_TLS").unwrap_or_default() != "true" {
                log::warn!("Rust implementation currently only supports plain TCP. Please set CELERIX_DISABLE_TLS=true.");
            }
            if let Ok(client) = Client::connect(&addr).await {
                return Ok(Arc::new(client));
            }
        }
    }

    let persistence = Arc::new(Persistence::new(data_dir)?);
    let initial_data = persistence.load_all()?;
    let store = MemStore::new(initial_data, Some(persistence));
    Ok(Arc::new(store))
}
