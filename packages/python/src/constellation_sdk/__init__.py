"""
Constellation Metagraph SDK.

Python SDK for standard operations on Constellation Network metagraphs built using the metakit framework.
"""

from importlib.metadata import version, PackageNotFoundError

try:
    __version__ = version("constellation-metagraph-sdk")
except PackageNotFoundError:
    __version__ = "0.0.0-dev"  # Fallback for development

# Core types
from .types import (
    ALGORITHM,
    CONSTELLATION_PREFIX,
    Hash,
    KeyPair,
    Signed,
    SignatureProof,
    VerificationResult,
)

# Canonicalization
from .canonicalize import canonicalize, canonicalize_bytes

# Binary encoding
from .binary import encode_data_update, to_bytes

# Hashing
from .hash import compute_digest, hash_bytes, hash_data

# Codec utilities
from .codec import decode_data_update

# Signing
from .sign import sign, sign_data_update, sign_hash

# Verification
from .verify import verify, verify_hash, verify_signature

# High-level API
from .signed_object import add_signature, batch_sign, create_signed_object

# Wallet utilities
from .wallet import (
    generate_key_pair,
    get_address,
    get_public_key_hex,
    get_public_key_id,
    is_valid_private_key,
    is_valid_public_key,
    key_pair_from_private_key,
)

__all__ = [
    # Version
    "__version__",
    # Types
    "SignatureProof",
    "Signed",
    "KeyPair",
    "Hash",
    "VerificationResult",
    # Constants
    "ALGORITHM",
    "CONSTELLATION_PREFIX",
    # Canonicalization
    "canonicalize",
    "canonicalize_bytes",
    # Binary
    "to_bytes",
    "encode_data_update",
    # Hash
    "hash_data",
    "hash_bytes",
    "compute_digest",
    # Codec
    "decode_data_update",
    # Sign
    "sign",
    "sign_data_update",
    "sign_hash",
    # Verify
    "verify",
    "verify_hash",
    "verify_signature",
    # High-level
    "create_signed_object",
    "add_signature",
    "batch_sign",
    # Wallet
    "generate_key_pair",
    "key_pair_from_private_key",
    "get_public_key_hex",
    "get_public_key_id",
    "get_address",
    "is_valid_private_key",
    "is_valid_public_key",
]
