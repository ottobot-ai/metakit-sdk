/**
 * Bridge Integration Tests for Delegation Flow
 * 
 * These tests specifically focus on the bridge service's handling of
 * delegation credentials and relayed transactions.
 * 
 * Tests are designed to FAIL initially (TDD approach) until bridge
 * delegation features are implemented.
 */

import { describe, it, expect, beforeEach, afterEach, beforeAll, afterAll } from 'jest';
import { 
  BridgeClient,
  DelegationValidationError,
  BridgeError,
  TransactionSubmissionResult
} from '../src/bridge';
import { 
  DelegationCredential,
  DelegationScope,
  DelegationType,
  createDelegation
} from '../src/delegation';
import { generateKeyPair, sign } from '../src';

describe('Bridge Delegation Integration', () => {
  let bridgeClient: BridgeClient;
  let userKeyPair: { privateKey: string; publicKey: string };
  let relayerKeyPair: { privateKey: string; publicKey: string };
  let delegation: DelegationCredential;

  beforeAll(() => {
    bridgeClient = new BridgeClient({
      metagraphUrl: 'http://localhost:9000',
      bridgeUrl: 'http://localhost:3000',
      timeout: 10000
    });

    userKeyPair = generateKeyPair();
    relayerKeyPair = generateKeyPair();
  });

  beforeEach(() => {
    delegation = null;
  });

  describe('Delegation Validation API', () => {
    it('should validate properly signed delegations', async () => {
      // Create valid delegation
      delegation = await createDelegation({
        delegator: userKeyPair.publicKey,
        delegate: relayerKeyPair.publicKey,
        scope: {
          type: DelegationScope.TRANSACTION,
          allowedActions: ['transfer'],
          maxAmount: '1000000000',
          validUntil: Date.now() + 3600000
        },
        type: DelegationType.TEMPORARY
      });

      const signedDelegation = await sign(delegation, userKeyPair.privateKey);

      // Bridge should validate delegation
      const result = await bridgeClient.validateDelegation(signedDelegation);

      expect(result.isValid).toBe(true);
      expect(result.error).toBeUndefined();
    });

    it('should reject delegations with invalid signatures', async () => {
      const invalidDelegation = {
        delegator: userKeyPair.publicKey,
        delegate: relayerKeyPair.publicKey,
        scope: {
          type: DelegationScope.TRANSACTION,
          allowedActions: ['transfer'],
          maxAmount: '1000000000',
          validUntil: Date.now() + 3600000
        },
        type: DelegationType.TEMPORARY,
        signature: 'invalid-signature-here'
      };

      const result = await bridgeClient.validateDelegation(invalidDelegation);

      expect(result.isValid).toBe(false);
      expect(result.error).toContain('signature');
    });

    it('should reject expired delegations', async () => {
      delegation = await createDelegation({
        delegator: userKeyPair.publicKey,
        delegate: relayerKeyPair.publicKey,
        scope: {
          type: DelegationScope.TRANSACTION,
          allowedActions: ['transfer'],
          maxAmount: '1000000000',
          validUntil: Date.now() - 1000 // Expired
        },
        type: DelegationType.TEMPORARY
      });

      const signedDelegation = await sign(delegation, userKeyPair.privateKey);

      const result = await bridgeClient.validateDelegation(signedDelegation);

      expect(result.isValid).toBe(false);
      expect(result.error).toContain('expired');
    });

    it('should validate delegation scope against transaction', async () => {
      // Create delegation with limited scope
      delegation = await createDelegation({
        delegator: userKeyPair.publicKey,
        delegate: relayerKeyPair.publicKey,
        scope: {
          type: DelegationScope.TRANSACTION,
          allowedActions: ['transfer'], // Only transfers allowed
          maxAmount: '500000000',
          validUntil: Date.now() + 3600000
        },
        type: DelegationType.TEMPORARY
      });

      const signedDelegation = await sign(delegation, userKeyPair.privateKey);

      // Test with valid transaction
      const validTransaction = {
        from: userKeyPair.publicKey,
        to: 'recipient-address',
        amount: '100000000',
        action: 'transfer'
      };

      const validationWithValidTx = await bridgeClient.validateDelegation({
        ...signedDelegation,
        proposedTransaction: validTransaction
      });

      expect(validationWithValidTx.isValid).toBe(true);

      // Test with invalid transaction (wrong action)
      const invalidTransaction = {
        from: userKeyPair.publicKey,
        to: 'contract-address',
        amount: '100000000',
        action: 'stateUpdate' // Not allowed by delegation
      };

      const validationWithInvalidTx = await bridgeClient.validateDelegation({
        ...signedDelegation,
        proposedTransaction: invalidTransaction
      });

      expect(validationWithInvalidTx.isValid).toBe(false);
      expect(validationWithInvalidTx.error).toContain('action not allowed');
    });
  });

  describe('Transaction Submission with Delegation', () => {
    it('should accept valid delegated transactions', async () => {
      delegation = await createDelegation({
        delegator: userKeyPair.publicKey,
        delegate: relayerKeyPair.publicKey,
        scope: {
          type: DelegationScope.TRANSACTION,
          allowedActions: ['transfer'],
          maxAmount: '1000000000',
          validUntil: Date.now() + 3600000
        },
        type: DelegationType.TEMPORARY
      });

      const signedDelegation = await sign(delegation, userKeyPair.privateKey);

      const transactionData = {
        from: userKeyPair.publicKey,
        to: 'recipient-address',
        amount: '100000000',
        action: 'transfer',
        nonce: 1,
        timestamp: Date.now(),
        delegation: signedDelegation,
        relayer: relayerKeyPair.publicKey
      };

      const result = await bridgeClient.submitTransaction(transactionData);

      expect(result.status).toBe('submitted');
      expect(result.transactionHash).toBeDefined();
    });

    it('should reject transactions without valid delegation', async () => {
      const transactionData = {
        from: userKeyPair.publicKey,
        to: 'recipient-address',
        amount: '100000000',
        action: 'transfer',
        nonce: 1,
        timestamp: Date.now(),
        // No delegation provided
        relayer: relayerKeyPair.publicKey
      };

      await expect(
        bridgeClient.submitTransaction(transactionData)
      ).rejects.toThrow(DelegationValidationError);
    });

    it('should reject transactions that exceed delegation limits', async () => {
      delegation = await createDelegation({
        delegator: userKeyPair.publicKey,
        delegate: relayerKeyPair.publicKey,
        scope: {
          type: DelegationScope.TRANSACTION,
          allowedActions: ['transfer'],
          maxAmount: '100000000', // Low limit
          validUntil: Date.now() + 3600000
        },
        type: DelegationType.TEMPORARY
      });

      const signedDelegation = await sign(delegation, userKeyPair.privateKey);

      const transactionData = {
        from: userKeyPair.publicKey,
        to: 'recipient-address',
        amount: '200000000', // Exceeds delegation limit
        action: 'transfer',
        nonce: 1,
        timestamp: Date.now(),
        delegation: signedDelegation,
        relayer: relayerKeyPair.publicKey
      };

      await expect(
        bridgeClient.submitTransaction(transactionData)
      ).rejects.toThrow(DelegationValidationError);
    });
  });

  describe('Delegation State Management', () => {
    it('should track delegation usage and remaining limits', async () => {
      delegation = await createDelegation({
        delegator: userKeyPair.publicKey,
        delegate: relayerKeyPair.publicKey,
        scope: {
          type: DelegationScope.TRANSACTION,
          allowedActions: ['transfer'],
          maxAmount: '1000000000',
          validUntil: Date.now() + 3600000
        },
        type: DelegationType.TEMPORARY
      });

      const signedDelegation = await sign(delegation, userKeyPair.privateKey);

      // Submit first transaction
      const tx1 = {
        from: userKeyPair.publicKey,
        to: 'recipient-1',
        amount: '300000000',
        action: 'transfer',
        nonce: 1,
        timestamp: Date.now(),
        delegation: signedDelegation,
        relayer: relayerKeyPair.publicKey
      };

      await bridgeClient.submitTransaction(tx1);

      // Check remaining delegation capacity
      const delegationStatus = await bridgeClient.getDelegationStatus(signedDelegation.id);

      expect(delegationStatus).toBeDefined();
      expect(delegationStatus.usedAmount).toBe('300000000');
      expect(delegationStatus.remainingAmount).toBe('700000000');
      expect(delegationStatus.transactionCount).toBe(1);
    });

    it('should handle delegation revocation', async () => {
      delegation = await createDelegation({
        delegator: userKeyPair.publicKey,
        delegate: relayerKeyPair.publicKey,
        scope: {
          type: DelegationScope.TRANSACTION,
          allowedActions: ['transfer'],
          maxAmount: '1000000000',
          validUntil: Date.now() + 3600000
        },
        type: DelegationType.TEMPORARY
      });

      const signedDelegation = await sign(delegation, userKeyPair.privateKey);

      // Initially delegation should be valid
      let result = await bridgeClient.validateDelegation(signedDelegation);
      expect(result.isValid).toBe(true);

      // Revoke delegation via bridge
      await bridgeClient.revokeDelegation({
        delegationId: signedDelegation.id,
        revokerSignature: await sign({ action: 'revoke', delegationId: signedDelegation.id }, userKeyPair.privateKey)
      });

      // Now delegation should be invalid
      result = await bridgeClient.validateDelegation(signedDelegation);
      expect(result.isValid).toBe(false);
      expect(result.error).toContain('revoked');
    });

    it('should support delegation queries and listing', async () => {
      // Create multiple delegations
      const delegations = [];
      for (let i = 0; i < 3; i++) {
        const del = await createDelegation({
          delegator: userKeyPair.publicKey,
          delegate: relayerKeyPair.publicKey,
          scope: {
            type: DelegationScope.TRANSACTION,
            allowedActions: ['transfer'],
            maxAmount: '1000000000',
            validUntil: Date.now() + 3600000 + (i * 1000) // Different expiry times
          },
          type: DelegationType.TEMPORARY
        });
        
        const signed = await sign(del, userKeyPair.privateKey);
        delegations.push(signed);
        
        // Submit delegation to bridge
        await bridgeClient.registerDelegation(signed);
      }

      // Query delegations for user
      const userDelegations = await bridgeClient.getDelegationsForUser(userKeyPair.publicKey);
      expect(userDelegations).toHaveLength(3);

      // Query delegations for relayer
      const relayerDelegations = await bridgeClient.getDelegationsForRelayer(relayerKeyPair.publicKey);
      expect(relayerDelegations).toHaveLength(3);

      // Query active delegations only
      const activeDelegations = await bridgeClient.getActiveDelegations({
        delegator: userKeyPair.publicKey,
        delegate: relayerKeyPair.publicKey
      });
      expect(activeDelegations).toHaveLength(3);
    });
  });

  describe('Bridge Error Handling', () => {
    it('should provide detailed error information for delegation failures', async () => {
      const malformedDelegation = {
        // Missing required fields
        delegator: userKeyPair.publicKey,
        // No delegate field
        scope: {
          type: 'invalid-scope-type',
          allowedActions: [],
          maxAmount: 'not-a-number',
          validUntil: 'invalid-timestamp'
        }
      };

      try {
        await bridgeClient.validateDelegation(malformedDelegation);
        fail('Should have thrown validation error');
      } catch (error) {
        expect(error).toBeInstanceOf(DelegationValidationError);
        expect(error.message).toContain('validation failed');
        expect(error.details).toBeDefined();
        expect(error.details.fields).toContain('delegate');
        expect(error.details.fields).toContain('scope.type');
        expect(error.details.fields).toContain('maxAmount');
      }
    });

    it('should handle bridge service errors gracefully', async () => {
      // Configure bridge client with invalid URL
      const faultyBridge = new BridgeClient({
        metagraphUrl: 'http://invalid-url:9999',
        bridgeUrl: 'http://invalid-bridge:3000',
        timeout: 1000
      });

      delegation = await createDelegation({
        delegator: userKeyPair.publicKey,
        delegate: relayerKeyPair.publicKey,
        scope: {
          type: DelegationScope.TRANSACTION,
          allowedActions: ['transfer'],
          maxAmount: '1000000000',
          validUntil: Date.now() + 3600000
        },
        type: DelegationType.TEMPORARY
      });

      const signedDelegation = await sign(delegation, userKeyPair.privateKey);

      await expect(
        faultyBridge.validateDelegation(signedDelegation)
      ).rejects.toThrow(BridgeError);
    });

    it('should handle rate limiting appropriately', async () => {
      delegation = await createDelegation({
        delegator: userKeyPair.publicKey,
        delegate: relayerKeyPair.publicKey,
        scope: {
          type: DelegationScope.TRANSACTION,
          allowedActions: ['transfer'],
          maxAmount: '1000000000',
          validUntil: Date.now() + 3600000
        },
        type: DelegationType.TEMPORARY
      });

      const signedDelegation = await sign(delegation, userKeyPair.privateKey);

      // Attempt many rapid requests to trigger rate limiting
      const rapidRequests = Array.from({ length: 100 }, () =>
        bridgeClient.validateDelegation(signedDelegation)
      );

      const results = await Promise.allSettled(rapidRequests);
      
      // Some requests should be rate limited
      const rateLimitedResults = results.filter(result => 
        result.status === 'rejected' && 
        (result.reason as Error).message.includes('rate limit')
      );

      expect(rateLimitedResults.length).toBeGreaterThan(0);
    }, 15000);
  });

  describe('Performance and Load Testing', () => {
    it('should handle concurrent delegation validations', async () => {
      // Create multiple delegations
      const delegations = await Promise.all(
        Array.from({ length: 50 }, async (_, i) => {
          const del = await createDelegation({
            delegator: userKeyPair.publicKey,
            delegate: relayerKeyPair.publicKey,
            scope: {
              type: DelegationScope.TRANSACTION,
              allowedActions: ['transfer'],
              maxAmount: '1000000000',
              validUntil: Date.now() + 3600000
            },
            type: DelegationType.TEMPORARY
          });
          
          return await sign(del, userKeyPair.privateKey);
        })
      );

      // Validate all concurrently
      const startTime = Date.now();
      const validationPromises = delegations.map(d => bridgeClient.validateDelegation(d));
      const results = await Promise.all(validationPromises);
      const endTime = Date.now();

      // All should be valid
      expect(results.every(r => r.isValid)).toBe(true);

      // Should complete reasonably quickly
      const avgTimePerValidation = (endTime - startTime) / delegations.length;
      expect(avgTimePerValidation).toBeLessThan(100); // Less than 100ms per validation
    }, 20000);
  });
});