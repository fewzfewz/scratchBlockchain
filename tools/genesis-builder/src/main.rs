//! # Genesis Builder CLI Tool
//!
//! Command-line tool for generating genesis.json files for blockchain initialization.

mod config;
mod validation;
mod builder;

use clap::{Parser, Subcommand};
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "genesis-builder")]
#[command(about = "Generate genesis.json for blockchain initialization")]
#[command(version = "1.0.0")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate genesis file from configuration
    Generate {
        /// Path to configuration file (TOML format)
        #[arg(short, long)]
        config: PathBuf,

        /// Output file path
        #[arg(short, long, default_value = "genesis.json")]
        output: PathBuf,
    },
    
    /// Validate configuration only (don't generate output)
    Validate {
        /// Path to configuration file (TOML format)
        #[arg(short, long)]
        config: PathBuf,
    },
    
    /// Show example configuration template
    Example,
    
    /// Show current configuration summary
    Show,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    println!("🔧 Genesis Builder - Modular Blockchain");
    println!("========================================\n");

    match args.command {
        Commands::Generate { config, output } => {
            println!("📄 Loading configuration from: {}", config.display());
            
            // Load configuration
            let config_content = fs::read_to_string(&config)?;
            let genesis_config = config::GenesisConfig::from_toml(&config_content)?;
            
            // Validate configuration
            println!("🔍 Validating configuration...");
            validation::Validator::validate_config(&genesis_config)?;
            println!("✅ Configuration valid\n");
            
            // Print summary
            validation::Validator::print_summary(&genesis_config)?;
            println!();
            
            // Build genesis
            println!("🏗️  Building genesis configuration...");
            let genesis = builder::GenesisBuilder::build(genesis_config)?;
            
            // Validate built genesis
            builder::GenesisBuilder::validate_built(&genesis)?;
            
            // Convert to JSON
            let json = builder::GenesisBuilder::to_json(&genesis)?;
            
            // Write output
            fs::write(&output, json)?;
            println!("✅ Genesis file generated: {}", output.display());
            
            // Print file size
            let metadata = fs::metadata(&output)?;
            println!("📦 File size: {} bytes", metadata.len());
        }
        
        Commands::Validate { config } => {
            println!("📄 Loading configuration from: {}", config.display());
            
            let config_content = fs::read_to_string(&config)?;
            let genesis_config = config::GenesisConfig::from_toml(&config_content)?;
            
            println!("🔍 Validating configuration...");
            validation::Validator::validate_config(&genesis_config)?;
            println!("✅ Configuration is valid!\n");
            
            validation::Validator::print_summary(&genesis_config)?;
            println!("\n✓ No issues found. Ready to generate genesis.");
        }
        
        Commands::Example => {
            println!("📝 Example configuration template:\n");
            println!("{}", config::GenesisConfig::example_toml());
            println!("\n💡 Save this to a .toml file and run:");
            println!("   genesis-builder generate --config config.toml");
        }
        
        Commands::Show => {
            println!("ℹ️  Genesis Builder Help");
            println!("=======================\n");
            println!("Commands:");
            println!("  generate --config <file> --output <file>  Generate genesis.json");
            println!("  validate --config <file>                  Validate config only");
            println!("  example                                   Show example config");
            println!("  show                                      Show this help");
            println!("\nExample workflow:");
            println!("  1. genesis-builder example > config.toml");
            println!("  2. Edit config.toml with your values");
            println!("  3. genesis-builder validate --config config.toml");
            println!("  4. genesis-builder generate --config config.toml --output genesis.json");
        }
    }

    Ok(())
}