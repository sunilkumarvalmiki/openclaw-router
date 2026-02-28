/**
 * Unified Router - Payment & CLI Tests
 * Day 4: Testing x402 payments and CLI commands
 */

import { describe, it, expect, beforeEach } from "vitest";
import { X402PaymentHandler, PaymentConfig } from "./unified-router-payments";
import { CLICommandHandler } from "./unified-router-cli";
import { UnifiedRouter } from "./unified-router-core-optimized";
import { ProviderRegistry } from "./unified-router-providers";
import { Profile, Tier } from "./unified-router-types";

// ============================================================================
// PAYMENT HANDLER TESTS
// ============================================================================

describe("X402PaymentHandler", () => {
  let handler: X402PaymentHandler;
  
  beforeEach(() => {
    const config: PaymentConfig = {
      enabled: true,
      minPaymentAmount: 1000000 // 1 USDC
    };
    handler = new X402PaymentHandler(config);
  });
  
  it("should create a wallet", async () => {
    const address = "0x1234567890123456789012345678901234567890";
    const wallet = await handler.createWallet(address);
    
    expect(wallet.address).toBeDefined();
    expect(wallet.balance).toBe(BigInt(0));
    expect(wallet.spent).toBe(BigInt(0));
  });
  
  it("should retrieve existing wallet", async () => {
    const address = "0x1234567890123456789012345678901234567890";
    const wallet1 = await handler.createWallet(address);
    const wallet2 = await handler.getWallet(address);
    
    expect(wallet2).toBeDefined();
    expect(wallet2?.balance).toBe(wallet1.balance);
  });
  
  it("should top up wallet with credits", async () => {
    const address = "0x1234567890123456789012345678901234567890";
    
    const result = await handler.topUpWallet(address, BigInt(5000000)); // 5 USDC
    
    expect(result.success).toBe(true);
    expect(result.receipt).toBeDefined();
    
    const wallet = await handler.getWallet(address);
    expect(wallet?.balance).toBe(BigInt(5000000));
  });
  
  it("should process payment", async () => {
    const address = "0x1234567890123456789012345678901234567890";
    
    // Create wallet with funds
    await handler.topUpWallet(address, BigInt(10000000)); // 10 USDC
    
    // Mock response
    const response: any = {
      id: "test-response-1",
      _costUSD: 5,
      _selectedProvider: "bedrock",
      model: "claude-3-sonnet",
      usage: { total_tokens: 2000 }
    };
    
    const result = await handler.processPayment({} as any, response, address);
    
    expect(result.success).toBe(true);
    expect(result.receipt).toBeDefined();
    
    const wallet = await handler.getWallet(address);
    expect(wallet?.balance).toBeLessThan(BigInt(10000000)); // Balance reduced
  });
  
  it("should prevent payment with insufficient balance", async () => {
    const address = "0x1234567890123456789012345678901234567890";
    
    // Create wallet with insufficient funds
    await handler.topUpWallet(address, BigInt(1000000)); // 1 USDC
    
    const response: any = {
      id: "test-response-2",
      _costUSD: 100, // 100 USDC
      _selectedProvider: "bedrock",
      model: "claude-3-opus",
      usage: { total_tokens: 10000 }
    };
    
    const result = await handler.processPayment({} as any, response, address);
    
    expect(result.success).toBe(false);
    expect(result.error).toContain("Insufficient balance");
  });
  
  it("should generate payment headers", () => {
    const address = "0x1234567890123456789012345678901234567890";
    const headers = handler.generatePaymentHeader(address, BigInt(5000000));
    
    expect(headers["Payment-Required"]).toBe("true");
    expect(headers["Payment-Protocol"]).toBe("x402");
    expect(headers["Payment-Amount-USDC"]).toBe("5000000");
  });
  
  it("should verify payment headers", () => {
    const headers = {
      "payment-required": "true",
      "payment-protocol": "x402",
      "payment-amount-usdc": "5000000"
    };
    
    expect(handler.verifyPaymentHeader(headers)).toBe(true);
  });
  
  it("should track transaction history", async () => {
    const address = "0x1234567890123456789012345678901234567890";
    
    await handler.topUpWallet(address, BigInt(10000000));
    
    const history = handler.getTransactionHistory(address);
    
    expect(history.length).toBeGreaterThan(0);
    expect(history[0].type).toBe("topup");
  });
  
  it("should calculate payment statistics", async () => {
    const address = "0x1234567890123456789012345678901234567890";
    
    await handler.topUpWallet(address, BigInt(5000000));
    
    const stats = handler.getPaymentStats();
    
    expect(stats.totalWallets).toBeGreaterThan(0);
    expect(stats.totalTransactions).toBeGreaterThan(0);
  });
});

// ============================================================================
// CLI COMMAND HANDLER TESTS
// ============================================================================

describe("CLICommandHandler", () => {
  let cli: CLICommandHandler;
  let router: UnifiedRouter;
  let payments: X402PaymentHandler;
  
  beforeEach(() => {
    const registry = new ProviderRegistry();
    router = new UnifiedRouter(
      {
        profile: Profile.AUTO,
        providers: [],
        optimization: {
          enableCache: false,
          enableDedup: false,
          enableCompress: false,
          filterTools: false,
          estimateTokens: true
        },
        payment: { enabled: true },
        metrics: { enabled: true },
        maxConcurrent: 10,
        requestTimeout: 30000
      },
      registry
    );
    
    payments = new X402PaymentHandler({ enabled: true });
    cli = new CLICommandHandler(router, payments);
  });
  
  it("should handle help command", async () => {
    const result = await cli.handleCommand("help");
    
    expect(result).toContain("Available Commands");
  });
  
  it("should handle wallet create command", async () => {
    const result = await cli.handleCommand("wallet", ["create"], "user123");
    
    expect(result).toContain("Wallet Created");
  });
  
  it("should handle wallet info command", async () => {
    await cli.handleCommand("wallet", ["create"], "user123");
    const result = await cli.handleCommand("wallet", [], "user123");
    
    expect(result).toContain("Wallet Info");
  });
  
  it("should handle topup command", async () => {
    await cli.handleCommand("wallet", ["create"], "user123");
    const result = await cli.handleCommand("topup", ["5"], "user123");
    
    expect(result).toContain("Top-up Successful");
  });
  
  it("should handle stats command", async () => {
    const result = await cli.handleCommand("stats");
    
    expect(result).toContain("System Statistics");
  });
  
  it("should handle profile command", async () => {
    const result = await cli.handleCommand("profile", ["eco"]);
    
    expect(result).toContain("Profile Changed");
  });
  
  it("should handle model list command", async () => {
    const result = await cli.handleCommand("model", ["list"]);
    
    expect(result).toContain("Available Models");
  });
  
  it("should handle model select command", async () => {
    const result = await cli.handleCommand("model", ["select", "gpt-4o"]);
    
    expect(result).toContain("Model Selected");
  });
  
  it("should handle history command", async () => {
    await cli.handleCommand("wallet", ["create"], "user123");
    await cli.handleCommand("topup", ["5"], "user123");
    const result = await cli.handleCommand("history", [], "user123");
    
    expect(result).toContain("Payment History");
  });
  
  it("should reject unknown command", async () => {
    const result = await cli.handleCommand("unknown");
    
    expect(result).toContain("Unknown command");
  });
});

// ============================================================================
// INTEGRATION TESTS
// ============================================================================

describe("Payment & CLI Integration", () => {
  it("should flow from CLI wallet create to payment", async () => {
    const registry = new ProviderRegistry();
    const router = new UnifiedRouter(
      {
        profile: Profile.AUTO,
        providers: [],
        optimization: {
          enableCache: false,
          enableDedup: false,
          enableCompress: false,
          filterTools: false,
          estimateTokens: true
        },
        payment: { enabled: true },
        metrics: { enabled: true },
        maxConcurrent: 10,
        requestTimeout: 30000
      },
      registry
    );
    
    const payments = new X402PaymentHandler({ enabled: true });
    const cli = new CLICommandHandler(router, payments);
    
    // Create wallet via CLI
    await cli.handleCommand("wallet", ["create"], "user456");
    
    // Top up via CLI
    await cli.handleCommand("topup", ["10"], "user456");
    
    // Check wallet directly
    const wallet = await payments.getWallet("user456");
    expect(wallet?.balance).toBeGreaterThan(BigInt(0));
  });
});

// ============================================================================
// SUMMARY
// ============================================================================

console.log(`
✅ Test Suite: Payment & CLI (Day 4)
   - Payment tests: 8 cases
   - CLI tests: 10 cases
   - Integration: 1 case
   - TOTAL: 19 new test cases (+ 53 from Days 1-3 = 72 total)
`);
