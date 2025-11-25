mod config;
mod validation;
mod builder;

use clap::Parser;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "genesis-builder")]
#[command(about = "Generate genesis.json for blockchain initialization", long_about = None)]
struct Args {
    /// Path to configuration file (TOML format)
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Output file path
    #[arg(short, long, default_value = "genesis.json")]
    output: PathBuf,

    /// Validate configuration only (don't generate output)
    #[arg(short, long)]
    validate: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("Genesis Builder - Modular Blockchain");
    println!("====================================\n");

    // Load configuration
    let config = if let Some(config_path) = args.config {
        println!("Loading configuration from: {}", config_path.display());
        let content = fs::read_to_string(&config_path)?;
        config::GenesisConfig::from_toml(&content)?
    } else {
        println!("Error: Configuration file required");
        println!("Usage: genesis-builder --config <path-to-config.toml>");
        std::process::exit(1);
    };

    // Validate configuration
    println!("Validating configuration...");
    validation::Validator::validate_config(&config)?;
    println!("✅ Configuration valid\n");

    // Print summary
    println!("Chain ID: {}", config.chain.chain_id);
    println!("Timestamp: {}", config.chain.timestamp);
    println!("Validators: {}", config.validators.len());
    println!("Accounts: {}", config.accounts.len());
    
    let total_stake: u128 = config.validators.iter()
        .filter_map(|v| v.stake.parse::<u128>().ok())
        .sum();
    let total_supply: u128 = config.accounts.iter()
        .filter_map(|a| a.balance.parse::<u128>().ok())
        .sum();
    println!("Total Stake: {}", total_stake);
    println!("Total Supply: {}\n", total_supply);

    if args.validate {
        println!("Validation complete! (--validate mode, no output generated)");
        return Ok(());
    }

    // Build genesis
    println!("Generating genesis configuration...");
    let genesis = builder::GenesisBuilder::build(config)?;
    let json = builder::GenesisBuilder::to_json(&genesis)?;

    // Write output
    fs::write(&args.output, json)?;
    println!("✅ Genesis file generated: {}", args.output.display());

    Ok(())
}
