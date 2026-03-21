/**
 * SimpleOrder Fiber Workflow Integration Tests
 * 
 * These tests extend the existing integration test coverage to include actual
 * fiber state transitions and workflow completion, not just creation and indexing.
 * 
 * These tests are designed to FAIL initially (TDD approach) until the SimpleOrder
 * fiber workflow and transition features are properly implemented in the bridge/ML0.
 * 
 * Acceptance Criteria from card:
 * - [ ] Create a SimpleOrder fiber (OPEN state)
 * - [ ] Execute transition: OPEN → COMPLETED (or CANCELLED)
 * - [ ] Verify state change reflected in ML0
 * - [ ] Verify state change reflected in indexer
 * - [ ] Assert no rejections for the transition
 */

import { describe, it, expect, beforeAll, afterAll, beforeEach, afterEach } from 'jest';
import { BridgeClient } from '../src/bridge';
import { generateKeyPair, sign } from '../src';

interface SimpleOrderData {
  orderId: string;
  item: string;
  quantity: number;
  price: string;
  customer: string;
  timestamp: number;
}

interface FiberState {
  fiberId: string;
  state: 'OPEN' | 'COMPLETED' | 'CANCELLED';
  data: SimpleOrderData;
  lastTransition: string | null;
  rejections: string[];
}

describe('SimpleOrder Fiber Workflow Integration', () => {
  let bridgeClient: BridgeClient;
  let agentKeyPair: { privateKey: string; publicKey: string };
  let testOrderData: SimpleOrderData;

  beforeAll(() => {
    // Initialize bridge client (assumes local development cluster)
    bridgeClient = new BridgeClient({
      metagraphUrl: 'http://localhost:9000', // ML0 endpoint
      bridgeUrl: 'http://localhost:3032',    // Bridge endpoint
      timeout: 10000
    });

    agentKeyPair = generateKeyPair();
    
    testOrderData = {
      orderId: `order-${Date.now()}`,
      item: 'Test Widget',
      quantity: 1,
      price: '100000000', // In base units
      customer: 'test-customer',
      timestamp: Date.now()
    };
  });

  beforeEach(() => {
    // Reset test state before each test
    testOrderData.orderId = `order-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  });

  describe('Agent Registration and Activation', () => {
    it('should register and activate agent before workflow tests', async () => {
      // Register agent
      const registrationResult = await bridgeClient.registerAgent({
        publicKey: agentKeyPair.publicKey,
        metadata: {
          name: 'Test Agent',
          description: 'Agent for SimpleOrder workflow testing'
        }
      });

      expect(registrationResult.success).toBe(true);

      // Activate agent
      const activationResult = await bridgeClient.activateAgent({
        publicKey: agentKeyPair.publicKey,
        signature: await sign({ action: 'activate' }, agentKeyPair.privateKey)
      });

      expect(activationResult.success).toBe(true);
    });
  });

  describe('SimpleOrder Fiber Creation', () => {
    it('should create SimpleOrder fiber in OPEN state', async () => {
      const fiberCreationData = {
        fiberType: 'SimpleOrder',
        initialState: 'OPEN',
        data: testOrderData,
        creator: agentKeyPair.publicKey
      };

      const signed = await sign(fiberCreationData, agentKeyPair.privateKey);

      const result = await bridgeClient.createFiber(signed);

      expect(result.success).toBe(true);
      expect(result.fiberId).toBeDefined();
      expect(result.state).toBe('OPEN');

      // Store fiber ID for subsequent tests
      testOrderData.orderId = result.fiberId;
    });

    it('should verify fiber appears in ML0 state', async () => {
      // First create the fiber
      const fiberCreationData = {
        fiberType: 'SimpleOrder',
        initialState: 'OPEN',
        data: testOrderData,
        creator: agentKeyPair.publicKey
      };

      const signed = await sign(fiberCreationData, agentKeyPair.privateKey);
      const createResult = await bridgeClient.createFiber(signed);
      
      expect(createResult.success).toBe(true);

      // Query ML0 for the fiber state
      const ml0State = await bridgeClient.getFiberFromML0(createResult.fiberId);

      expect(ml0State).toBeDefined();
      expect(ml0State.state).toBe('OPEN');
      expect(ml0State.data).toMatchObject(testOrderData);
      expect(ml0State.rejections).toHaveLength(0);
    });

    it('should verify fiber appears in indexer', async () => {
      // Create fiber
      const fiberCreationData = {
        fiberType: 'SimpleOrder',
        initialState: 'OPEN',
        data: testOrderData,
        creator: agentKeyPair.publicKey
      };

      const signed = await sign(fiberCreationData, agentKeyPair.privateKey);
      const createResult = await bridgeClient.createFiber(signed);
      
      expect(createResult.success).toBe(true);

      // Wait for indexer propagation
      await new Promise(resolve => setTimeout(resolve, 2000));

      // Query indexer for the fiber
      const indexerResult = await bridgeClient.queryIndexer({
        fiberId: createResult.fiberId
      });

      expect(indexerResult).toBeDefined();
      expect(indexerResult.state).toBe('OPEN');
      expect(indexerResult.fiberType).toBe('SimpleOrder');
    });
  });

  describe('SimpleOrder State Transitions', () => {
    let createdFiberId: string;

    beforeEach(async () => {
      // Create a fresh fiber for each transition test
      const fiberCreationData = {
        fiberType: 'SimpleOrder',
        initialState: 'OPEN',
        data: testOrderData,
        creator: agentKeyPair.publicKey
      };

      const signed = await sign(fiberCreationData, agentKeyPair.privateKey);
      const createResult = await bridgeClient.createFiber(signed);
      
      expect(createResult.success).toBe(true);
      createdFiberId = createResult.fiberId;
    });

    it('should execute transition OPEN → COMPLETED', async () => {
      // Execute transition to COMPLETED state
      const transitionData = {
        fiberId: createdFiberId,
        transition: 'complete_order',
        transitionData: {
          completedAt: Date.now(),
          completedBy: agentKeyPair.publicKey,
          reason: 'Order fulfilled successfully'
        }
      };

      const signed = await sign(transitionData, agentKeyPair.privateKey);
      const transitionResult = await bridgeClient.executeTransition(signed);

      expect(transitionResult.success).toBe(true);
      expect(transitionResult.newState).toBe('COMPLETED');
      expect(transitionResult.rejections).toHaveLength(0);
    });

    it('should execute transition OPEN → CANCELLED', async () => {
      // Execute transition to CANCELLED state
      const transitionData = {
        fiberId: createdFiberId,
        transition: 'cancel_order',
        transitionData: {
          cancelledAt: Date.now(),
          cancelledBy: agentKeyPair.publicKey,
          reason: 'Customer requested cancellation'
        }
      };

      const signed = await sign(transitionData, agentKeyPair.privateKey);
      const transitionResult = await bridgeClient.executeTransition(signed);

      expect(transitionResult.success).toBe(true);
      expect(transitionResult.newState).toBe('CANCELLED');
      expect(transitionResult.rejections).toHaveLength(0);
    });

    it('should verify state change reflected in ML0 after COMPLETED transition', async () => {
      // Execute transition
      const transitionData = {
        fiberId: createdFiberId,
        transition: 'complete_order',
        transitionData: {
          completedAt: Date.now(),
          completedBy: agentKeyPair.publicKey,
          reason: 'Test completion'
        }
      };

      const signed = await sign(transitionData, agentKeyPair.privateKey);
      await bridgeClient.executeTransition(signed);

      // Wait for ML0 propagation
      await new Promise(resolve => setTimeout(resolve, 1000));

      // Query ML0 for updated state
      const ml0State = await bridgeClient.getFiberFromML0(createdFiberId);

      expect(ml0State.state).toBe('COMPLETED');
      expect(ml0State.lastTransition).toBe('complete_order');
      expect(ml0State.rejections).toHaveLength(0);
    });

    it('should verify state change reflected in indexer after COMPLETED transition', async () => {
      // Execute transition
      const transitionData = {
        fiberId: createdFiberId,
        transition: 'complete_order',
        transitionData: {
          completedAt: Date.now(),
          completedBy: agentKeyPair.publicKey
        }
      };

      const signed = await sign(transitionData, agentKeyPair.privateKey);
      await bridgeClient.executeTransition(signed);

      // Wait for indexer propagation
      await new Promise(resolve => setTimeout(resolve, 3000));

      // Query indexer for updated state
      const indexerResult = await bridgeClient.queryIndexer({
        fiberId: createdFiberId
      });

      expect(indexerResult.state).toBe('COMPLETED');
      expect(indexerResult.transitionHistory).toContain('complete_order');
    });

    it('should verify state change reflected in indexer after CANCELLED transition', async () => {
      // Execute transition
      const transitionData = {
        fiberId: createdFiberId,
        transition: 'cancel_order',
        transitionData: {
          cancelledAt: Date.now(),
          reason: 'Test cancellation'
        }
      };

      const signed = await sign(transitionData, agentKeyPair.privateKey);
      await bridgeClient.executeTransition(signed);

      // Wait for indexer propagation
      await new Promise(resolve => setTimeout(resolve, 3000));

      // Query indexer for updated state
      const indexerResult = await bridgeClient.queryIndexer({
        fiberId: createdFiberId
      });

      expect(indexerResult.state).toBe('CANCELLED');
      expect(indexerResult.transitionHistory).toContain('cancel_order');
    });
  });

  describe('Transition Rejection Handling', () => {
    let createdFiberId: string;

    beforeEach(async () => {
      const fiberCreationData = {
        fiberType: 'SimpleOrder',
        initialState: 'OPEN',
        data: testOrderData,
        creator: agentKeyPair.publicKey
      };

      const signed = await sign(fiberCreationData, agentKeyPair.privateKey);
      const createResult = await bridgeClient.createFiber(signed);
      createdFiberId = createResult.fiberId;
    });

    it('should assert no rejections for valid transitions', async () => {
      const transitionData = {
        fiberId: createdFiberId,
        transition: 'complete_order',
        transitionData: {
          completedAt: Date.now(),
          completedBy: agentKeyPair.publicKey
        }
      };

      const signed = await sign(transitionData, agentKeyPair.privateKey);
      const result = await bridgeClient.executeTransition(signed);

      expect(result.rejections).toHaveLength(0);
      expect(result.success).toBe(true);
    });

    it('should handle invalid transitions gracefully', async () => {
      // Try to execute an invalid transition
      const invalidTransitionData = {
        fiberId: createdFiberId,
        transition: 'invalid_transition',
        transitionData: {}
      };

      const signed = await sign(invalidTransitionData, agentKeyPair.privateKey);
      
      await expect(
        bridgeClient.executeTransition(signed)
      ).rejects.toThrow(/invalid.*transition/i);
    });

    it('should prevent transitions on completed orders', async () => {
      // First complete the order
      const completeData = {
        fiberId: createdFiberId,
        transition: 'complete_order',
        transitionData: { completedAt: Date.now() }
      };

      const completeSigned = await sign(completeData, agentKeyPair.privateKey);
      await bridgeClient.executeTransition(completeSigned);

      // Try to cancel already completed order
      const cancelData = {
        fiberId: createdFiberId,
        transition: 'cancel_order',
        transitionData: { cancelledAt: Date.now() }
      };

      const cancelSigned = await sign(cancelData, agentKeyPair.privateKey);
      
      await expect(
        bridgeClient.executeTransition(cancelSigned)
      ).rejects.toThrow(/already.*completed|invalid.*state/i);
    });
  });

  describe('End-to-End Workflow Verification', () => {
    it('should complete full SimpleOrder lifecycle: OPEN → COMPLETED', async () => {
      // Step 1: Create fiber
      const fiberData = {
        fiberType: 'SimpleOrder',
        initialState: 'OPEN',
        data: testOrderData,
        creator: agentKeyPair.publicKey
      };

      const createSigned = await sign(fiberData, agentKeyPair.privateKey);
      const createResult = await bridgeClient.createFiber(createSigned);
      
      expect(createResult.success).toBe(true);
      expect(createResult.state).toBe('OPEN');

      // Step 2: Execute transition
      const transitionData = {
        fiberId: createResult.fiberId,
        transition: 'complete_order',
        transitionData: {
          completedAt: Date.now(),
          completedBy: agentKeyPair.publicKey
        }
      };

      const transitionSigned = await sign(transitionData, agentKeyPair.privateKey);
      const transitionResult = await bridgeClient.executeTransition(transitionSigned);
      
      expect(transitionResult.success).toBe(true);
      expect(transitionResult.newState).toBe('COMPLETED');

      // Step 3: Verify final state in ML0
      await new Promise(resolve => setTimeout(resolve, 1000));
      const finalML0State = await bridgeClient.getFiberFromML0(createResult.fiberId);
      
      expect(finalML0State.state).toBe('COMPLETED');
      expect(finalML0State.rejections).toHaveLength(0);

      // Step 4: Verify final state in indexer
      await new Promise(resolve => setTimeout(resolve, 2000));
      const finalIndexerState = await bridgeClient.queryIndexer({
        fiberId: createResult.fiberId
      });
      
      expect(finalIndexerState.state).toBe('COMPLETED');
    });

    it('should complete full SimpleOrder lifecycle: OPEN → CANCELLED', async () => {
      // Step 1: Create fiber
      const fiberData = {
        fiberType: 'SimpleOrder',
        initialState: 'OPEN',
        data: testOrderData,
        creator: agentKeyPair.publicKey
      };

      const createSigned = await sign(fiberData, agentKeyPair.privateKey);
      const createResult = await bridgeClient.createFiber(createSigned);
      
      expect(createResult.success).toBe(true);

      // Step 2: Execute cancellation
      const cancelData = {
        fiberId: createResult.fiberId,
        transition: 'cancel_order',
        transitionData: {
          cancelledAt: Date.now(),
          reason: 'End-to-end test cancellation'
        }
      };

      const cancelSigned = await sign(cancelData, agentKeyPair.privateKey);
      const cancelResult = await bridgeClient.executeTransition(cancelSigned);
      
      expect(cancelResult.success).toBe(true);
      expect(cancelResult.newState).toBe('CANCELLED');

      // Step 3: Verify final state consistency
      await new Promise(resolve => setTimeout(resolve, 3000));
      
      const ml0State = await bridgeClient.getFiberFromML0(createResult.fiberId);
      const indexerState = await bridgeClient.queryIndexer({
        fiberId: createResult.fiberId
      });
      
      expect(ml0State.state).toBe('CANCELLED');
      expect(indexerState.state).toBe('CANCELLED');
      expect(ml0State.rejections).toHaveLength(0);
    });
  });

  describe('Performance and Load Considerations', () => {
    it('should handle multiple simultaneous fiber transitions', async () => {
      // Create multiple fibers
      const fibers = [];
      for (let i = 0; i < 5; i++) {
        const orderData = {
          ...testOrderData,
          orderId: `load-test-${i}-${Date.now()}`
        };

        const fiberData = {
          fiberType: 'SimpleOrder',
          initialState: 'OPEN',
          data: orderData,
          creator: agentKeyPair.publicKey
        };

        const signed = await sign(fiberData, agentKeyPair.privateKey);
        const result = await bridgeClient.createFiber(signed);
        fibers.push(result.fiberId);
      }

      // Execute transitions on all fibers simultaneously
      const transitionPromises = fibers.map(async (fiberId, index) => {
        const transitionData = {
          fiberId,
          transition: index % 2 === 0 ? 'complete_order' : 'cancel_order',
          transitionData: {
            timestamp: Date.now(),
            executor: agentKeyPair.publicKey
          }
        };

        const signed = await sign(transitionData, agentKeyPair.privateKey);
        return bridgeClient.executeTransition(signed);
      });

      const results = await Promise.all(transitionPromises);

      // All transitions should succeed
      results.forEach((result, index) => {
        expect(result.success).toBe(true);
        expect(result.rejections).toHaveLength(0);
        expect(result.newState).toBe(index % 2 === 0 ? 'COMPLETED' : 'CANCELLED');
      });
    }, 30000); // Extended timeout for load test
  });
});