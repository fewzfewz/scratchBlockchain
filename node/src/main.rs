use clap::{Parser, Subcommand};
use consensus::SimpleConsensus;
use network::NetworkService;
use storage::MemStore;
use tracing::info;

#[derive(Parser)]
#[command(name = "modular-node")]
#[command(about = "A modular blockchain node", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the node
    Start,
    /// Generate a new keypair
    KeyGen,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match &cli.command {
        Commands::Start => {
            info!("Starting Modular Blockchain Node...");

            // Initialize components
            let store = MemStore::new();
            let _consensus = SimpleConsensus::new(vec![]); // No validators for now
            let (network, _network_cmd_sender) = NetworkService::new()?;

            info!("Components initialized. Running network...");
            network.run().await;
        }
        Commands::KeyGen => {
            println!("Generating keypair...");
            // TODO: Use actual key generation logic from network/common
            println!("Keypair generated (mock)");
        }
    }

    Ok(())
}
