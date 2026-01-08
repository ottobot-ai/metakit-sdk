# Constellation Metagraph SDK

Official multi-language SDK for Constellation Metagraph signing operations.

[![CI](https://github.com/Constellation-Labs/metakit-sdk/actions/workflows/ci.yml/badge.svg)](https://github.com/Constellation-Labs/metakit-sdk/actions/workflows/ci.yml)
[![npm version](https://img.shields.io/npm/v/@constellation-network/metagraph-sdk.svg)](https://www.npmjs.com/package/@constellation-network/metagraph-sdk)
[![PyPI version](https://img.shields.io/pypi/v/constellation-metagraph-sdk.svg)](https://pypi.org/project/constellation-metagraph-sdk/)

## Packages

| Language | Package | Documentation |
|----------|---------|---------------|
| TypeScript | [@constellation-network/metagraph-sdk](https://www.npmjs.com/package/@constellation-network/metagraph-sdk) | [README](./packages/typescript/README.md) |
| Python | [constellation-metagraph-sdk](https://pypi.org/project/constellation-metagraph-sdk/) | [README](./packages/python/README.md) |

## Features

- **RFC 8785 Canonicalization**: Deterministic JSON encoding for consistent hashing
- **ECDSA on secp256k1**: Industry-standard cryptographic signatures compatible with Constellation Network
- **Multi-signature support**: Create and verify multi-party signatures
- **Cross-language compatibility**: Signatures created in one language verify in all others
- **DataUpdate support**: Sign data for direct submission to metagraph data-l1 endpoints

## Quick Start

### TypeScript

```bash
npm install @constellation-network/metagraph-sdk @stardust-collective/dag4
```

```typescript
import { createSignedObject, verify, generateKeyPair } from '@constellation-network/metagraph-sdk';

// Generate a key pair
const keyPair = generateKeyPair();
console.log('Address:', keyPair.address);

// Sign data
const data = { action: 'UPDATE', payload: { key: 'value' } };
const signed = await createSignedObject(data, keyPair.privateKey);

// Verify
const result = await verify(signed);
console.log('Valid:', result.isValid);
```

### Python

```bash
pip install constellation-metagraph-sdk
```

```python
from constellation_sdk import create_signed_object, verify, generate_key_pair

# Generate a key pair
key_pair = generate_key_pair()
print(f'Address: {key_pair.address}')

# Sign data
data = {'action': 'UPDATE', 'payload': {'key': 'value'}}
signed = create_signed_object(data, key_pair.private_key)

# Verify
result = verify(signed)
print(f'Valid: {result.is_valid}')
```

## Development

### Prerequisites

- Node.js 18+
- Python 3.10+
- Make (standard on Linux/macOS)

### Setup

```bash
# Clone the repository
git clone https://github.com/Constellation-Labs/metakit-sdk.git
cd metakit-sdk

# Install all dependencies
make install

# Or install individually
make install-ts
make install-py
```

### Common Commands

```bash
make help        # Show all available commands

make test        # Run all tests
make test-ts     # Run TypeScript tests only
make test-py     # Run Python tests only

make lint        # Lint all packages
make format      # Format all code
make build       # Build all packages
make clean       # Clean build artifacts
```

## Cross-Language Compatibility

All SDKs are validated against shared test vectors in `/shared/test_vectors.json`. This ensures:

- Canonicalization produces identical output across all languages
- Hashing produces identical digests
- Signatures created in one language verify in all others

## Releasing

Releases are triggered by pushing tags:

```bash
# TypeScript release
git tag -a typescript-v1.0.0 -m "TypeScript SDK v1.0.0"
git push origin typescript-v1.0.0

# Python release
git tag -a python-v1.0.0 -m "Python SDK v1.0.0"
git push origin python-v1.0.0
```

See [docs/PUBLISHING.md](./docs/PUBLISHING.md) for complete setup instructions.

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

## License

Apache-2.0