# E2E Currency Transaction Tests

End-to-end test scripts for submitting currency transactions to a local metagraph. Each script demonstrates the complete flow: loading config, deriving addresses, fetching last reference, creating/signing transactions, and submitting to the network.

## Prerequisites

- A running local metagraph with Currency L1 available
- A funded wallet (private key with metagraph tokens)

## Configuration

All scripts read from `config.json` in the e2e directory:

```json
{
  "private_key": "YOUR_64_CHAR_HEX_PRIVATE_KEY",
  "destination": "DAG_DESTINATION_ADDRESS",
  "amount": 1.0,
  "fee": 0.0,
  "currency_l1_url": "http://localhost:9300"
}
```

Copy `config.example.json` to `config.json` and fill in your values.

| Field | Description |
|-------|-------------|
| `private_key` | 64-character hex private key for the sender wallet |
| `destination` | DAG address to send tokens to |
| `amount` | Amount in tokens (e.g., `1.0` = 1 token) |
| `fee` | Transaction fee in tokens (usually `0.0`) |
| `currency_l1_url` | Currency L1 endpoint URL |

## Scripts

### Python

```bash
cd python

# Generate a new keypair
python3 send_currency_tx.py --generate-keypair

# Send a transaction
python3 send_currency_tx.py

# Use a different config file
python3 send_currency_tx.py --config ../other_config.json
```

**Requirements:** Python 3.10+, uses the SDK from `packages/python/src`

---

### TypeScript

```bash
cd typescript

# Generate a new keypair
npx tsx send_currency_tx.ts --generate-keypair

# Send a transaction
npx tsx send_currency_tx.ts

# Use a different config file
npx tsx send_currency_tx.ts --config ../other_config.json
```

**Requirements:** Node.js 18+, uses the SDK from `packages/typescript/src`

---

### Go

```bash
cd go

# Generate a new keypair
go run send_currency_tx.go -generate-keypair

# Send a transaction
go run send_currency_tx.go

# Use a different config file
go run send_currency_tx.go -config ../other_config.json
```

**Requirements:** Go 1.18+, uses the SDK from `packages/go` via `go.mod` replace directive

---

### Rust

```bash
cd rust

# Generate a new keypair
cargo run --release -- --generate-keypair

# Send a transaction
cargo run --release

# Use a different config file
cargo run --release -- --config ../other_config.json
```

**Requirements:** Rust 1.70+, uses the SDK from `packages/rust` via `Cargo.toml` path dependency

---

### Java

First, install the SDK to your local Maven repository:

```bash
cd ../packages/java
mvn install -DskipTests
```

Then run the script:

```bash
cd java

# Generate a new keypair
mvn exec:java -Dexec.args="--generate-keypair" -q

# Send a transaction
mvn exec:java -q

# Use a different config file
mvn exec:java -Dexec.args="--config ../other_config.json" -q
```

**Requirements:** Java 11+, Maven 3.6+

---

## Common Options

All scripts support these flags:

| Flag | Description |
|------|-------------|
| `--help` / `-h` | Show usage information |
| `--generate-keypair` | Generate a new random keypair and exit |
| `--config <file>` | Path to config file (default: `../config.json`) |

## Output

On success, each script outputs:

1. Source and destination addresses
2. Node health check
3. Last transaction reference (hash + ordinal)
4. Transaction creation confirmation
5. Signature verification
6. Submitted transaction hash
7. Transaction status (Waiting/InProgress/Accepted)
