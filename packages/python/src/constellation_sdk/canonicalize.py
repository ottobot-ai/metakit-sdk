"""
RFC 8785 JSON Canonicalization.

Provides deterministic JSON serialization according to RFC 8785.
This ensures identical JSON objects always produce identical strings.
"""

from typing import Any

import rfc8785


def canonicalize(data: Any) -> str:
    """
    Canonicalize JSON data according to RFC 8785.

    Key features:
    - Object keys sorted by UTF-16BE binary comparison
    - Numbers serialized in shortest decimal representation
    - No whitespace
    - Proper Unicode escaping

    Args:
        data: Any JSON-serializable object (dict, list, str, int, float, bool, None)

    Returns:
        Canonical JSON string

    Example:
        >>> canonicalize({"b": 2, "a": 1})
        '{"a":1,"b":2}'
    """
    # rfc8785.dumps returns bytes, decode to string
    return rfc8785.dumps(data).decode("utf-8")


def canonicalize_bytes(data: Any) -> bytes:
    """
    Canonicalize JSON data and return as UTF-8 bytes.

    Args:
        data: Any JSON-serializable object

    Returns:
        Canonical JSON as UTF-8 encoded bytes
    """
    return rfc8785.dumps(data)
