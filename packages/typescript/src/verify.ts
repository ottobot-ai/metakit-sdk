/**
 * Signature Verification
 *
 * Verify ECDSA signatures using secp256k1 curve.
 */

import { sha256 } from 'js-sha256';
import { sha512 } from 'js-sha512';
import { ec as EC } from 'elliptic';
import { Signed, SignatureProof, VerificationResult } from './types';
import { toBytes } from './binary';

// Initialize secp256k1 curve
const ec = new EC('secp256k1');

/**
 * Verify a signed object
 *
 * @param signed - Signed object with value and proofs
 * @param isDataUpdate - Whether the value was signed as a DataUpdate
 * @returns VerificationResult with valid/invalid proof lists
 *
 * @example
 * ```typescript
 * const result = await verify(signedObject);
 * if (result.isValid) {
 *   console.log('All signatures valid');
 * }
 * ```
 */
export async function verify<T>(
  signed: Signed<T>,
  isDataUpdate: boolean = false
): Promise<VerificationResult> {
  // Compute the hash that should have been signed
  const bytes = toBytes(signed.value, isDataUpdate);
  const hashHex = sha256.hex(bytes);

  const validProofs: SignatureProof[] = [];
  const invalidProofs: SignatureProof[] = [];

  for (const proof of signed.proofs) {
    try {
      const isValid = await verifyHash(hashHex, proof.signature, proof.id);
      if (isValid) {
        validProofs.push(proof);
      } else {
        invalidProofs.push(proof);
      }
    } catch {
      // Verification error = invalid
      invalidProofs.push(proof);
    }
  }

  return {
    isValid: invalidProofs.length === 0 && validProofs.length > 0,
    validProofs,
    invalidProofs,
  };
}

/**
 * Verify a signature against a SHA-256 hash
 *
 * Protocol:
 * 1. Treat hash hex as UTF-8 bytes (NOT hex decode)
 * 2. SHA-512 hash
 * 3. Truncate to 32 bytes
 * 4. Verify ECDSA signature
 *
 * @param hashHex - SHA-256 hash as 64-character hex string
 * @param signature - DER-encoded signature in hex format
 * @param publicKeyId - Public key in hex (with or without 04 prefix)
 * @returns true if signature is valid
 */
export async function verifyHash(
  hashHex: string,
  signature: string,
  publicKeyId: string
): Promise<boolean> {
  try {
    // Step 1-2: Hash hex as UTF-8, then SHA-512
    const hexAsUtf8 = new TextEncoder().encode(hashHex);
    const sha512Hash = sha512.array(hexAsUtf8);

    // Step 3: Truncate to 32 bytes
    const truncatedHash = new Uint8Array(sha512Hash.slice(0, 32));

    // Normalize public key (add 04 prefix if needed)
    const fullPublicKey = normalizePublicKey(publicKeyId);

    // Step 4: Verify with elliptic
    const key = ec.keyFromPublic(fullPublicKey, 'hex');
    return key.verify(truncatedHash, signature);
  } catch {
    return false;
  }
}

/**
 * Verify a single signature proof against data
 *
 * @param data - The original data that was signed
 * @param proof - The signature proof to verify
 * @param isDataUpdate - Whether data was signed as DataUpdate
 * @returns true if signature is valid
 */
export async function verifySignature<T>(
  data: T,
  proof: SignatureProof,
  isDataUpdate: boolean = false
): Promise<boolean> {
  const bytes = toBytes(data, isDataUpdate);
  const hashHex = sha256.hex(bytes);
  return verifyHash(hashHex, proof.signature, proof.id);
}

/**
 * Normalize public key to full format (with 04 prefix)
 */
function normalizePublicKey(publicKey: string): string {
  // If 128 chars (without 04 prefix), add prefix
  if (publicKey.length === 128) {
    return '04' + publicKey;
  }
  // If 130 chars (with 04 prefix), return as-is
  if (publicKey.length === 130 && publicKey.startsWith('04')) {
    return publicKey;
  }
  // Otherwise return as-is
  return publicKey;
}
