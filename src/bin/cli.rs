use celerix_store::sdk;
use clap::{Parser, Subcommand};
use serde_json::Value;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "data")]
    data_dir: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Clone)]
enum Commands {
    Get { persona: String, app: String, key: String },
    Set { persona: String, app: String, key: String, value: String },
    Del { persona: String, app: String, key: String },
    ListPersonas,
    ListApps { persona: String },
    Dump { persona: String, app: String },
    Move { src_persona: String, dst_persona: String, app: String, key: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let store = sdk::new(&cli.data_dir).await?;

    match cli.command {
        Commands::Get { persona, app, key } => {
            let val = store.get(&persona, &app, &key).await?;
            println!("{}", serde_json::to_string_pretty(&val)?);
        }
        Commands::Set { persona, app, key, value } => {
            let val: Value = serde_json::from_str(&value).unwrap_or(Value::String(value));
            store.set(&persona, &app, &key, val).await?;
            println!("OK");
        }
        Commands::Del { persona, app, key } => {
            store.delete(&persona, &app, &key).await?;
            println!("OK");
        }
        Commands::ListPersonas => {
            let list = store.get_personas().await?;
            println!("{}", serde_json::to_string_pretty(&list)?);
        }
        Commands::ListApps { persona } => {
            let list = store.get_apps(&persona).await?;
            println!("{}", serde_json::to_string_pretty(&list)?);
        }
        Commands::Dump { persona, app } => {
            let data = store.get_app_store(&persona, &app).await?;
            println!("{}", serde_json::to_string_pretty(&data)?);
        }
        Commands::Move { src_persona, dst_persona, app, key } => {
            store.move_key(&src_persona, &dst_persona, &app, &key).await?;
            println!("OK");
        }
    }

    Ok(())
}
