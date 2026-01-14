//! Simple script to send a currency transaction to a local metagraph.
//!
//! Reads configuration from config.json and submits a currency transaction
//! to the Currency L1 endpoint.
//!
//! Usage:
//!   cargo run
//!   cargo run -- --config other_config.json
//!   cargo run -- --generate-keypair

use clap::Parser;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::process;

use constellation_sdk::{
    create_currency_transaction, key_pair_from_private_key, verify_currency_transaction,
    wallet::generate_key_pair, CurrencyTransaction, TransactionReference, TransferParams,
};
use constellation_sdk::network::{CurrencyL1Client, NetworkConfig};

#[derive(Deserialize)]
struct Config {
    private_key: String,
    destination: String,
    amount: f64,
    #[serde(default)]
    fee: f64,
    currency_l1_url: String,
}

#[derive(Parser)]
#[command(name = "send_currency_tx")]
#[command(about = "Send a currency transaction to a local metagraph")]
struct Args {
    /// Path to config file
    #[arg(short, long, default_value = "../config.json")]
    config: PathBuf,

    /// Generate a new keypair and exit
    #[arg(long)]
    generate_keypair: bool,
}

fn generate_keypair_command() {
    let keypair = generate_key_pair();
    println!("Generated new keypair:");
    println!("  Private Key: {}", keypair.private_key);
    println!("  Public Key:  {}", keypair.public_key);
    println!("  DAG Address: {}", keypair.address);
    println!("\nSave the private key to your config.json to use it for transactions.");
}

async fn send_transaction(config: Config) {
    let private_key = &config.private_key;
    let destination = &config.destination;
    let amount = config.amount;
    let fee = config.fee;
    let currency_l1_url = &config.currency_l1_url;

    // Validate private key format
    if private_key == "YOUR_64_CHAR_HEX_PRIVATE_KEY_HERE" {
        eprintln!("Error: Please set your private key in config.json");
        eprintln!("Run with --generate-keypair to create a new keypair");
        process::exit(1);
    }

    if private_key.len() != 64 {
        eprintln!(
            "Error: Private key must be 64 hex characters, got {}",
            private_key.len()
        );
        process::exit(1);
    }

    // Derive address from private key
    let keypair = match key_pair_from_private_key(private_key) {
        Ok(kp) => kp,
        Err(e) => {
            eprintln!("Error deriving keypair: {}", e);
            process::exit(1);
        }
    };
    let source_address = &keypair.address;

    println!("Source Address: {}", source_address);
    println!("Destination:    {}", destination);
    println!("Amount:         {} tokens", amount);
    println!("Fee:            {} tokens", fee);
    println!("Currency L1:    {}", currency_l1_url);
    println!();

    // Create client
    let network_config = NetworkConfig {
        l1_url: Some(currency_l1_url.clone()),
        ..Default::default()
    };
    let client = match CurrencyL1Client::new(network_config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error creating client: {}", e);
            process::exit(1);
        }
    };

    // Check health
    println!("Checking node health...");
    if !client.check_health().await {
        eprintln!("Error: Currency L1 node is not responding");
        process::exit(1);
    }
    println!("Node is healthy!");
    println!();

    // Get last reference
    println!("Fetching last reference for {}...", source_address);
    let last_ref = match client.get_last_reference(source_address).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error getting last reference: {}", e);
            process::exit(1);
        }
    };
    println!("Last Reference Hash:    {}", last_ref.hash);
    println!("Last Reference Ordinal: {}", last_ref.ordinal);
    println!();

    // Create transaction
    println!("Creating transaction...");
    let transfer_params = TransferParams {
        destination: destination.clone(),
        amount,
        fee,
    };
    let tx: CurrencyTransaction = match create_currency_transaction(
        transfer_params,
        private_key,
        TransactionReference {
            hash: last_ref.hash,
            ordinal: last_ref.ordinal,
        },
    ) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error creating transaction: {}", e);
            process::exit(1);
        }
    };
    println!("Transaction created successfully!");

    // Verify transaction locally
    println!("Verifying transaction signature...");
    let result = verify_currency_transaction(&tx);
    if !result.is_valid {
        eprintln!("Error: Transaction signature verification failed!");
        process::exit(1);
    }
    println!("Signature verified!");
    println!();

    // Submit transaction
    println!("Submitting transaction to network...");
    let response = match client.post_transaction(&tx).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error submitting transaction: {}", e);
            process::exit(1);
        }
    };
    println!("Transaction submitted!");
    println!("Transaction Hash: {}", response.hash);
    println!();

    // Poll for status (optional)
    println!("Checking transaction status...");
    match client.get_pending_transaction(&response.hash).await {
        Ok(Some(pending)) => {
            println!("Status: {}", pending.status);
        }
        Ok(None) => {
            println!("Transaction not found in pending pool (may already be confirmed)");
        }
        Err(e) => {
            println!("Could not check status: {}", e);
        }
    }

    println!();
    println!("Done!");
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if args.generate_keypair {
        generate_keypair_command();
        return;
    }

    // Load config
    let config_path = args.config;
    let config_content = match fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error: Config file not found: {:?} ({})", config_path, e);
            process::exit(1);
        }
    };

    let config: Config = match serde_json::from_str(&config_content) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error parsing config: {}", e);
            process::exit(1);
        }
    };

    send_transaction(config).await;
}
