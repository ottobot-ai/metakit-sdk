#!/usr/bin/env npx tsx
/**
 * Simple script to send a currency transaction to a local metagraph.
 *
 * Reads configuration from config.json and submits a currency transaction
 * to the Currency L1 endpoint.
 *
 * Usage:
 *   npx tsx send_currency_tx.ts
 *   npx tsx send_currency_tx.ts --config other_config.json
 *   npx tsx send_currency_tx.ts --generate-keypair
 */

import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

// Import from the SDK (assuming built)
import {
  CurrencyL1Client,
  createCurrencyTransaction,
  generateKeyPair,
  keyPairFromPrivateKey,
  verifyCurrencyTransaction,
  type TransferParams,
} from '../../packages/typescript/src/index';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

interface Config {
  private_key: string;
  destination: string;
  amount: number;
  fee?: number;
  currency_l1_url: string;
}

function loadConfig(configPath: string): Config {
  const content = readFileSync(configPath, 'utf-8');
  return JSON.parse(content);
}

function generateKeypairCommand(): void {
  const keypair = generateKeyPair();
  console.log('Generated new keypair:');
  console.log(`  Private Key: ${keypair.privateKey}`);
  console.log(`  Public Key:  ${keypair.publicKey}`);
  console.log(`  DAG Address: ${keypair.address}`);
  console.log('\nSave the private key to your config.json to use it for transactions.');
}

async function sendTransaction(config: Config): Promise<void> {
  // Validate config
  const requiredFields = ['private_key', 'destination', 'amount', 'currency_l1_url'];
  for (const field of requiredFields) {
    if (!(field in config)) {
      console.error(`Error: Missing required field '${field}' in config`);
      process.exit(1);
    }
  }

  const privateKey = config.private_key;
  const destination = config.destination;
  const amount = config.amount;
  const fee = config.fee ?? 0;
  const currencyL1Url = config.currency_l1_url;

  // Validate private key format
  if (privateKey === 'YOUR_64_CHAR_HEX_PRIVATE_KEY_HERE') {
    console.error('Error: Please set your private key in config.json');
    console.error('Run with --generate-keypair to create a new keypair');
    process.exit(1);
  }

  if (privateKey.length !== 64) {
    console.error(`Error: Private key must be 64 hex characters, got ${privateKey.length}`);
    process.exit(1);
  }

  // Derive address from private key
  const keypair = keyPairFromPrivateKey(privateKey);
  const sourceAddress = keypair.address;

  console.log(`Source Address: ${sourceAddress}`);
  console.log(`Destination:    ${destination}`);
  console.log(`Amount:         ${amount} tokens`);
  console.log(`Fee:            ${fee} tokens`);
  console.log(`Currency L1:    ${currencyL1Url}`);
  console.log();

  // Create client
  const client = new CurrencyL1Client({ l1Url: currencyL1Url });

  // Check health
  console.log('Checking node health...');
  const isHealthy = await client.checkHealth();
  if (!isHealthy) {
    console.error('Error: Currency L1 node is not responding');
    process.exit(1);
  }
  console.log('Node is healthy!');
  console.log();

  // Get last reference
  console.log(`Fetching last reference for ${sourceAddress}...`);
  let lastRef;
  try {
    lastRef = await client.getLastReference(sourceAddress);
    console.log(`Last Reference Hash:    ${lastRef.hash}`);
    console.log(`Last Reference Ordinal: ${lastRef.ordinal}`);
  } catch (e) {
    console.error(`Error getting last reference: ${e}`);
    process.exit(1);
  }
  console.log();

  // Create transaction
  console.log('Creating transaction...');
  let tx;
  try {
    const transferParams: TransferParams = {
      destination,
      amount,
      fee,
    };
    tx = await createCurrencyTransaction(transferParams, privateKey, lastRef);
    console.log('Transaction created successfully!');
  } catch (e) {
    console.error(`Error creating transaction: ${e}`);
    process.exit(1);
  }

  // Verify transaction locally
  console.log('Verifying transaction signature...');
  const result = await verifyCurrencyTransaction(tx);
  if (!result.isValid) {
    console.error('Error: Transaction signature verification failed!');
    process.exit(1);
  }
  console.log('Signature verified!');
  console.log();

  // Submit transaction
  console.log('Submitting transaction to network...');
  let response;
  try {
    response = await client.postTransaction(tx);
    console.log('Transaction submitted!');
    console.log(`Transaction Hash: ${response.hash}`);
  } catch (e) {
    console.error(`Error submitting transaction: ${e}`);
    process.exit(1);
  }
  console.log();

  // Poll for status (optional)
  console.log('Checking transaction status...');
  try {
    const pending = await client.getPendingTransaction(response.hash);
    if (pending) {
      console.log(`Status: ${pending.status}`);
    } else {
      console.log('Transaction not found in pending pool (may already be confirmed)');
    }
  } catch (e) {
    console.log(`Could not check status: ${e}`);
  }

  console.log();
  console.log('Done!');
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);

  // Parse arguments
  let configFile = 'config.json';
  let generateKeypair = false;

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--generate-keypair') {
      generateKeypair = true;
    } else if (args[i] === '--config' && args[i + 1]) {
      configFile = args[++i];
    } else if (args[i] === '--help' || args[i] === '-h') {
      console.log('Usage: npx tsx send_currency_tx.ts [options]');
      console.log();
      console.log('Options:');
      console.log('  --config <file>     Path to config file (default: config.json)');
      console.log('  --generate-keypair  Generate a new keypair and exit');
      console.log('  --help, -h          Show this help message');
      process.exit(0);
    }
  }

  if (generateKeypair) {
    generateKeypairCommand();
    return;
  }

  // Resolve config path relative to script directory
  const configPath = resolve(__dirname, configFile);

  try {
    readFileSync(configPath);
  } catch {
    console.error(`Error: Config file not found: ${configPath}`);
    process.exit(1);
  }

  const config = loadConfig(configPath);
  await sendTransaction(config);
}

main().catch((e) => {
  console.error('Unexpected error:', e);
  process.exit(1);
});
