/**
 * Signing Functions
 *
 * ECDSA signing using secp256k1 curve via dag4js.
 * Implements the Constellation signature protocol.
 */

import { dag4 } from '@stardust-collective/dag4';
import { sha256 } from 'js-sha256';
import { SignatureProof } from './types';
import { canonicalize } from './canonicalize';
import { toBytes } from './binary';

/**
 * Sign data using the regular Constellation protocol (non-DataUpdate)
 *
 * Protocol:
 * 1. Canonicalize JSON
 * 2. UTF-8 encode
 * 3. SHA-256 hash
 * 4. Sign using dag4.keyStore.sign (handles SHA-512 + truncate + ECDSA internally)
 *
 * @param data - Any JSON-serializable object
 * @param privateKey - Private key in hex format
 * @returns SignatureProof with public key ID and signature
 *
 * @example
 * ```typescript
 * const proof = await sign({ action: 'test' }, privateKeyHex);
 * console.log(proof.id);        // public key (128 chars)
 * console.log(proof.signature); // DER signature
 * ```
 */
export async function sign<T>(data: T, privateKey: string): Promise<SignatureProof> {
  // Serialize and hash
  const bytes = toBytes(data, false);
  const hashHex = sha256.hex(bytes);

  // Sign the hash
  const signature = await signHash(hashHex, privateKey);

  // Get public key
  const publicKey = dag4.keyStore.getPublicKeyFromPrivate(privateKey, false);
  const id = normalizePublicKeyId(publicKey);

  return { id, signature };
}

/**
 * Sign data as a DataUpdate (with Constellation prefix)
 *
 * Uses dag4.keyStore.dataSign which handles the DataUpdate-specific encoding.
 *
 * @param data - Any JSON-serializable object
 * @param privateKey - Private key in hex format
 * @returns SignatureProof
 */
export async function signDataUpdate<T>(data: T, privateKey: string): Promise<SignatureProof> {
  // Canonicalize and base64 encode for dataSign
  const canonicalJson = canonicalize(data);
  const base64String = Buffer.from(canonicalJson, 'utf-8').toString('base64');

  // Use dag4's dataSign which handles the Constellation prefix internally
  const signature = await dag4.keyStore.dataSign(privateKey, base64String);

  // Get public key
  const publicKey = dag4.keyStore.getPublicKeyFromPrivate(privateKey, false);
  const id = normalizePublicKeyId(publicKey);

  return { id, signature };
}

/**
 * Sign a pre-computed SHA-256 hash
 *
 * The hash should be a 64-character hex string.
 * dag4.keyStore.sign internally:
 * 1. Treats hex as UTF-8 bytes
 * 2. SHA-512 hashes
 * 3. Truncates to 32 bytes
 * 4. Signs with ECDSA
 *
 * @param hashHex - SHA-256 hash as 64-character hex string
 * @param privateKey - Private key in hex format
 * @returns DER-encoded signature in hex format
 */
export async function signHash(hashHex: string, privateKey: string): Promise<string> {
  return dag4.keyStore.sign(privateKey, hashHex);
}

/**
 * Normalize public key to ID format (without 04 prefix, 128 chars)
 */
function normalizePublicKeyId(publicKey: string): string {
  // If 130 chars (with 04 prefix), remove prefix
  if (publicKey.length === 130 && publicKey.startsWith('04')) {
    return publicKey.substring(2);
  }
  // If 128 chars (without prefix), return as-is
  if (publicKey.length === 128) {
    return publicKey;
  }
  // Otherwise return as-is and let validation catch issues
  return publicKey;
}
