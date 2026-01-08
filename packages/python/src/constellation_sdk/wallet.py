"""
Wallet and Key Management Utilities.

Functions for generating and managing cryptographic keys.
"""

import hashlib
import re

from ecdsa import SECP256k1, SigningKey

from .types import KeyPair


def generate_key_pair() -> KeyPair:
    """
    Generate a new random key pair.

    Returns:
        KeyPair with private key, public key, and DAG address

    Example:
        >>> key_pair = generate_key_pair()
        >>> print(key_pair.address)    # DAG address
        >>> print(key_pair.private_key) # 64 char hex
        >>> print(key_pair.public_key)  # 130 char hex (with 04 prefix)
    """
    sk = SigningKey.generate(curve=SECP256k1)
    private_key = sk.to_string().hex()
    return key_pair_from_private_key(private_key)


def key_pair_from_private_key(private_key: str) -> KeyPair:
    """
    Derive a key pair from an existing private key.

    Args:
        private_key: Private key in hex format (64 characters)

    Returns:
        KeyPair with private key, public key, and DAG address

    Example:
        >>> key_pair = key_pair_from_private_key(existing_private_key)
    """
    sk = SigningKey.from_string(bytes.fromhex(private_key), curve=SECP256k1)
    vk = sk.verifying_key

    # Get uncompressed public key (with 04 prefix)
    x_coord = vk.pubkey.point.x()
    y_coord = vk.pubkey.point.y()
    public_key = f"04{x_coord:064x}{y_coord:064x}"

    # Derive DAG address
    address = get_address(public_key)

    return KeyPair(private_key=private_key, public_key=public_key, address=address)


def get_public_key_hex(private_key: str, compressed: bool = False) -> str:
    """
    Get the public key hex from a private key.

    Args:
        private_key: Private key in hex format
        compressed: If True, returns compressed public key (33 bytes)

    Returns:
        Public key in hex format
    """
    sk = SigningKey.from_string(bytes.fromhex(private_key), curve=SECP256k1)
    vk = sk.verifying_key

    x_coord = vk.pubkey.point.x()
    y_coord = vk.pubkey.point.y()

    if compressed:
        prefix = "02" if y_coord % 2 == 0 else "03"
        return f"{prefix}{x_coord:064x}"
    else:
        return f"04{x_coord:064x}{y_coord:064x}"


def get_public_key_id(private_key: str) -> str:
    """
    Get the public key ID (without 04 prefix) from a private key.

    This format is used in SignatureProof.id

    Args:
        private_key: Private key in hex format

    Returns:
        Public key ID (128 characters, no 04 prefix)
    """
    public_key = get_public_key_hex(private_key, compressed=False)
    # Remove 04 prefix
    return public_key[2:]


def get_address(public_key: str) -> str:
    """
    Get DAG address from a public key.

    Uses Constellation's address derivation:
    1. SHA-256 hash of public key bytes
    2. Convert to base58
    3. Prepend "DAG"

    Args:
        public_key: Public key in hex format (with or without 04 prefix)

    Returns:
        DAG address string
    """
    # Normalize public key
    if public_key.startswith("04"):
        pk_hex = public_key[2:]
    else:
        pk_hex = public_key

    # SHA-256 hash of public key bytes
    pk_bytes = bytes.fromhex(pk_hex)
    hash_bytes = hashlib.sha256(pk_bytes).digest()

    # Base58 encode
    base58_encoded = _base58_encode(hash_bytes)

    # Return with DAG prefix
    return f"DAG{base58_encoded}"


def is_valid_private_key(private_key: str) -> bool:
    """
    Validate that a private key is correctly formatted.

    Args:
        private_key: Private key to validate

    Returns:
        True if valid hex string of correct length
    """
    if not isinstance(private_key, str):
        return False
    if len(private_key) != 64:
        return False
    return bool(re.match(r"^[0-9a-fA-F]+$", private_key))


def is_valid_public_key(public_key: str) -> bool:
    """
    Validate that a public key is correctly formatted.

    Args:
        public_key: Public key to validate

    Returns:
        True if valid hex string of correct length
    """
    if not isinstance(public_key, str):
        return False
    # With 04 prefix: 130 chars, without: 128 chars
    if len(public_key) not in (128, 130):
        return False
    return bool(re.match(r"^[0-9a-fA-F]+$", public_key))


# Base58 alphabet (Bitcoin/Constellation style)
_BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"


def _base58_encode(data: bytes) -> str:
    """Base58 encode bytes."""
    # Count leading zeros
    leading_zeros = 0
    for byte in data:
        if byte == 0:
            leading_zeros += 1
        else:
            break

    # Convert to integer
    num = int.from_bytes(data, "big")

    # Encode
    result = ""
    while num > 0:
        num, remainder = divmod(num, 58)
        result = _BASE58_ALPHABET[remainder] + result

    # Add leading '1's for each leading zero byte
    return "1" * leading_zeros + result
