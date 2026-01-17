use std::sync::Arc;
use celerix_store::{engine::{MemStore, Persistence}, AppEnumeration};
use celerix_store::server::Router;
use clap::Parser;
use std::env;
use tokio::signal;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    data_dir: Option<String>,

    #[arg(short, long)]
    port: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    let data_dir = args.data_dir
        .or_else(|| env::var("CELERIX_DATA_DIR").ok())
        .unwrap_or_else(|| "data".to_string());

    let port = args.port
        .or_else(|| env::var("CELERIX_PORT").ok())
        .unwrap_or_else(|| "7001".to_string());

    let persistence = Arc::new(Persistence::new(&data_dir)?);
    let initial_data = persistence.load_all()?;
    let store = Arc::new(MemStore::new(initial_data, Some(persistence)));

    let router = Router::new(store.clone());
    
    println!("Starting Celerix Store Daemon...");
    println!("Engine started. Loaded {} personas.", store.get_personas().await?.len());
    println!("Celerix Engine listening on :{} (TCP)", port);

    tokio::select! {
        res = router.listen(&port) => {
            if let Err(e) = res {
                eprintln!("TCP Server failed: {}", e);
            }
        }
        _ = signal::ctrl_c() => {
            println!("\nShutdown signal received. Finalizing disk writes...");
            store.wait().await;
            println!("Persistence complete. Exiting.");
        }
    }

    Ok(())
}
