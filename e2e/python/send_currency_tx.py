#!/usr/bin/env python3
"""
Simple script to send a currency transaction to a local metagraph.

Reads configuration from config.json and submits a currency transaction
to the Currency L1 endpoint.

Usage:
    python send_currency_tx.py
    python send_currency_tx.py --config other_config.json
    python send_currency_tx.py --generate-keypair
"""

import argparse
import json
import sys
from pathlib import Path

# Add the SDK to the path
sdk_path = Path(__file__).parent.parent.parent / "packages" / "python" / "src"
sys.path.insert(0, str(sdk_path))

from constellation_sdk import (
    CurrencyL1Client,
    NetworkConfig,
    TransferParams,
    create_currency_transaction,
    generate_key_pair,
    key_pair_from_private_key,
    verify_currency_transaction,
)


def load_config(config_path: str) -> dict:
    """Load configuration from JSON file."""
    with open(config_path) as f:
        return json.load(f)


def generate_keypair_command():
    """Generate a new keypair and print it."""
    keypair = generate_key_pair()
    print("Generated new keypair:")
    print(f"  Private Key: {keypair.private_key}")
    print(f"  Public Key:  {keypair.public_key}")
    print(f"  DAG Address: {keypair.address}")
    print("\nSave the private key to your config.json to use it for transactions.")


def send_transaction(config: dict):
    """Create and send a currency transaction."""
    # Validate config
    required_fields = ["private_key", "destination", "amount", "currency_l1_url"]
    for field in required_fields:
        if field not in config:
            print(f"Error: Missing required field '{field}' in config")
            sys.exit(1)

    private_key = config["private_key"]
    destination = config["destination"]
    amount = float(config["amount"])
    fee = float(config.get("fee", 0.0))
    currency_l1_url = config["currency_l1_url"]

    # Validate private key format
    if private_key == "YOUR_64_CHAR_HEX_PRIVATE_KEY_HERE":
        print("Error: Please set your private key in config.json")
        print("Run with --generate-keypair to create a new keypair")
        sys.exit(1)

    if len(private_key) != 64:
        print(f"Error: Private key must be 64 hex characters, got {len(private_key)}")
        sys.exit(1)

    # Derive address from private key
    keypair = key_pair_from_private_key(private_key)
    source_address = keypair.address

    print(f"Source Address: {source_address}")
    print(f"Destination:    {destination}")
    print(f"Amount:         {amount} tokens")
    print(f"Fee:            {fee} tokens")
    print(f"Currency L1:    {currency_l1_url}")
    print()

    # Create client
    client = CurrencyL1Client(NetworkConfig(l1_url=currency_l1_url))

    # Check health
    print("Checking node health...")
    if not client.check_health():
        print("Error: Currency L1 node is not responding")
        sys.exit(1)
    print("Node is healthy!")
    print()

    # Get last reference
    print(f"Fetching last reference for {source_address}...")
    try:
        last_ref = client.get_last_reference(source_address)
        print(f"Last Reference Hash:    {last_ref.hash}")
        print(f"Last Reference Ordinal: {last_ref.ordinal}")
    except Exception as e:
        print(f"Error getting last reference: {e}")
        sys.exit(1)
    print()

    # Create transaction
    print("Creating transaction...")
    try:
        transfer_params = TransferParams(
            destination=destination,
            amount=amount,
            fee=fee,
        )
        tx = create_currency_transaction(transfer_params, private_key, last_ref)
        print("Transaction created successfully!")
    except Exception as e:
        print(f"Error creating transaction: {e}")
        sys.exit(1)

    # Verify transaction locally
    print("Verifying transaction signature...")
    result = verify_currency_transaction(tx)
    if not result.is_valid:
        print("Error: Transaction signature verification failed!")
        sys.exit(1)
    print("Signature verified!")
    print()

    # Submit transaction
    print("Submitting transaction to network...")
    try:
        response = client.post_transaction(tx)
        print(f"Transaction submitted!")
        print(f"Transaction Hash: {response.hash}")
    except Exception as e:
        print(f"Error submitting transaction: {e}")
        sys.exit(1)
    print()

    # Poll for status (optional)
    print("Checking transaction status...")
    try:
        pending = client.get_pending_transaction(response.hash)
        if pending:
            print(f"Status: {pending.status}")
        else:
            print("Transaction not found in pending pool (may already be confirmed)")
    except Exception as e:
        print(f"Could not check status: {e}")

    print()
    print("Done!")


def main():
    parser = argparse.ArgumentParser(
        description="Send a currency transaction to a local metagraph"
    )
    parser.add_argument(
        "--config",
        default="config.json",
        help="Path to config file (default: config.json)",
    )
    parser.add_argument(
        "--generate-keypair",
        action="store_true",
        help="Generate a new keypair and exit",
    )
    args = parser.parse_args()

    if args.generate_keypair:
        generate_keypair_command()
        return

    # Resolve config path relative to script directory
    script_dir = Path(__file__).parent
    config_path = script_dir / args.config

    if not config_path.exists():
        print(f"Error: Config file not found: {config_path}")
        sys.exit(1)

    config = load_config(str(config_path))
    send_transaction(config)


if __name__ == "__main__":
    main()
