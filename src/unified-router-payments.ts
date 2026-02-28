/**
 * Unified Router - x402 USDC Payment Integration
 * Day 4: Blockchain-based payment handling via x402 protocol
 * 
 * Features:
 * - x402 protocol compliance (HTTP payment semantics)
 * - USDC payment processing
 * - Wallet management
 * - Credit tracking
 * - Invoice generation
 * - Payment verification
 */

import { ethers } from "ethers";
import { LLMRequest, LLMResponse } from "./unified-router-types";

/**
 * Payment Configuration
 */
export interface PaymentConfig {
  enabled: boolean;
  provider?: string; // Ethereum RPC provider URL
  usdcTokenAddress?: string;
  walletPrivateKey?: string;
  paymentReceiverAddress?: string;
  minPaymentAmount?: number; // In wei
}

/**
 * User Wallet & Credit Management
 */
export interface UserWallet {
  address: string;
  balance: bigint; // In wei (USDC has 6 decimals)
  credits: number; // Credits remaining
  spent: bigint; // Total spent
  transactions: PaymentTransaction[];
  lastTopUp?: Date;
  createdAt: Date;
}

/**
 * Payment Transaction Record
 */
export interface PaymentTransaction {
  id: string;
  hash?: string; // Transaction hash (if on-chain)
  type: "payment" | "refund" | "topup";
  amount: bigint; // In wei
  costUSD: number; // For reference
  requestId: string;
  timestamp: Date;
  status: "pending" | "confirmed" | "failed";
  metadata?: Record<string, any>;
}

/**
 * Payment Receipt / Invoice
 */
export interface PaymentReceipt {
  invoiceId: string;
  walletAddress: string;
  requestId: string;
  costUSD: number;
  amountUSDC: bigint;
  timestamp: Date;
  txHash?: string;
  status: "pending" | "completed" | "failed";
  metadata?: Record<string, any>;
}

/**
 * x402 Payment Handler
 * Implements x402 protocol with USDC payments
 */
export class X402PaymentHandler {
  private config: PaymentConfig;
  private wallets: Map<string, UserWallet> = new Map();
  private transactions: Map<string, PaymentTransaction> = new Map();
  private provider?: ethers.JsonRpcProvider;
  private usdcContract?: ethers.Contract;
  private minPaymentAmount: bigint;
  
  constructor(config: PaymentConfig) {
    this.config = config;
    this.minPaymentAmount = BigInt(config.minPaymentAmount || 1000000); // 1 USDC (6 decimals)
    
    if (config.enabled) {
      this.initializeBlockchain();
    }
  }
  
  /**
   * Initialize blockchain connection
   */
  private initializeBlockchain(): void {
    try {
      const rpcUrl = this.config.provider || "https://eth-mainnet.g.alchemy.com/v2/demo";
      this.provider = new ethers.JsonRpcProvider(rpcUrl);
      
      // USDC contract ABI (ERC-20 minimal)
      const usdcAbi = [
        "function balanceOf(address owner) public view returns (uint256)",
        "function transfer(address to, uint256 amount) public returns (bool)",
        "function approve(address spender, uint256 amount) public returns (bool)",
        "function allowance(address owner, address spender) public view returns (uint256)",
        "event Transfer(address indexed from, address indexed to, uint256 value)",
        "event Approval(address indexed owner, address indexed spender, uint256 value)"
      ];
      
      // Polygon USDC address (or Ethereum, configurable)
      const usdcAddress = this.config.usdcTokenAddress || "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"; // Ethereum USDC
      
      if (this.config.walletPrivateKey) {
        const signer = new ethers.Wallet(this.config.walletPrivateKey, this.provider);
        this.usdcContract = new ethers.Contract(usdcAddress, usdcAbi, signer);
      } else {
        this.usdcContract = new ethers.Contract(usdcAddress, usdcAbi, this.provider);
      }
      
      console.log("✅ x402 Payment Handler initialized");
    } catch (error) {
      console.error("❌ Failed to initialize blockchain:", error);
    }
  }
  
  /**
   * Create or retrieve user wallet
   */
  async createWallet(address: string): Promise<UserWallet> {
    // Normalize address
    const normalizedAddress = ethers.getAddress(address);
    
    if (this.wallets.has(normalizedAddress)) {
      return this.wallets.get(normalizedAddress)!;
    }
    
    const wallet: UserWallet = {
      address: normalizedAddress,
      balance: BigInt(0),
      credits: 0,
      spent: BigInt(0),
      transactions: [],
      createdAt: new Date()
    };
    
    this.wallets.set(normalizedAddress, wallet);
    console.log(`✅ Wallet created: ${normalizedAddress}`);
    
    return wallet;
  }
  
  /**
   * Get wallet info
   */
  async getWallet(address: string): Promise<UserWallet | null> {
    const normalizedAddress = ethers.getAddress(address);
    
    const wallet = this.wallets.get(normalizedAddress);
    if (!wallet) return null;
    
    // Update on-chain balance
    if (this.provider && this.usdcContract) {
      try {
        const onChainBalance = await this.usdcContract.balanceOf(normalizedAddress);
        wallet.balance = BigInt(onChainBalance);
      } catch (error) {
        console.warn("Could not fetch on-chain balance");
      }
    }
    
    return wallet;
  }
  
  /**
   * Process payment for a request
   * Returns x402 payment header if needed
   */
  async processPayment(
    request: LLMRequest,
    response: LLMResponse,
    walletAddress: string
  ): Promise<{ success: boolean; receipt?: PaymentReceipt; error?: string }> {
    if (!this.config.enabled) {
      return { success: true }; // Payments disabled
    }
    
    try {
      const normalizedAddress = ethers.getAddress(walletAddress);
      
      // Get or create wallet
      let wallet = this.wallets.get(normalizedAddress);
      if (!wallet) {
        wallet = await this.createWallet(walletAddress);
      }
      
      // Calculate cost
      const costUSD = response._costUSD || 0;
      const amountUSDC = this.usdToUsdc(costUSD);
      
      // Check balance
      if (wallet.balance < amountUSDC) {
        return {
          success: false,
          error: `Insufficient balance. Need ${this.formatUsdc(amountUSDC)}, have ${this.formatUsdc(wallet.balance)}`
        };
      }
      
      // Create transaction record
      const txId = this.generateTransactionId();
      const transaction: PaymentTransaction = {
        id: txId,
        type: "payment",
        amount: amountUSDC,
        costUSD,
        requestId: response.id,
        timestamp: new Date(),
        status: "pending",
        metadata: {
          model: response.model,
          tokens: response.usage?.total_tokens || 0
        }
      };
      
      // Store transaction
      this.transactions.set(txId, transaction);
      wallet.transactions.push(transaction);
      
      // Deduct from wallet (off-chain for speed)
      wallet.balance -= amountUSDC;
      wallet.spent += amountUSDC;
      wallet.credits += Math.floor(Number(amountUSDC) / 1000000); // 1 USDC = 1 credit
      
      // Create receipt
      const receipt: PaymentReceipt = {
        invoiceId: this.generateInvoiceId(),
        walletAddress: normalizedAddress,
        requestId: response.id,
        costUSD,
        amountUSDC,
        timestamp: new Date(),
        status: "completed",
        metadata: {
          model: response.model,
          provider: response._selectedProvider,
          tokens: response.usage?.total_tokens
        }
      };
      
      // Update transaction status
      transaction.status = "confirmed";
      
      console.log(`✅ Payment processed: ${this.formatUsdc(amountUSDC)} USDC`);
      
      return { success: true, receipt };
      
    } catch (error: any) {
      console.error("❌ Payment processing failed:", error.message);
      return {
        success: false,
        error: `Payment failed: ${error.message}`
      };
    }
  }
  
  /**
   * Process on-chain USDC transfer (optional, for on-chain settlement)
   */
  async processOnChainPayment(
    walletAddress: string,
    amountUSDC: bigint
  ): Promise<{ success: boolean; txHash?: string; error?: string }> {
    if (!this.config.enabled || !this.usdcContract || !this.config.walletPrivateKey) {
      return { success: false, error: "On-chain payments not configured" };
    }
    
    try {
      const receiverAddress = this.config.paymentReceiverAddress;
      if (!receiverAddress) {
        return { success: false, error: "Payment receiver address not configured" };
      }
      
      // Send USDC
      const tx = await this.usdcContract.transfer(receiverAddress, amountUSDC);
      const receipt = await tx.wait();
      
      console.log(`✅ On-chain payment sent: ${tx.hash}`);
      
      return { success: true, txHash: tx.hash };
      
    } catch (error: any) {
      console.error("❌ On-chain payment failed:", error.message);
      return {
        success: false,
        error: `On-chain payment failed: ${error.message}`
      };
    }
  }
  
  /**
   * Top up wallet with credits
   */
  async topUpWallet(
    address: string,
    amountUSDC: bigint
  ): Promise<{ success: boolean; receipt?: PaymentReceipt; error?: string }> {
    try {
      const normalizedAddress = ethers.getAddress(address);
      
      let wallet = this.wallets.get(normalizedAddress);
      if (!wallet) {
        wallet = await this.createWallet(normalizedAddress);
      }
      
      // Add credits
      wallet.balance += amountUSDC;
      wallet.lastTopUp = new Date();
      
      // Record transaction
      const transaction: PaymentTransaction = {
        id: this.generateTransactionId(),
        type: "topup",
        amount: amountUSDC,
        costUSD: 0,
        requestId: "topup",
        timestamp: new Date(),
        status: "confirmed"
      };
      
      this.transactions.set(transaction.id, transaction);
      wallet.transactions.push(transaction);
      
      const receipt: PaymentReceipt = {
        invoiceId: this.generateInvoiceId(),
        walletAddress: normalizedAddress,
        requestId: "topup",
        costUSD: 0,
        amountUSDC,
        timestamp: new Date(),
        status: "completed"
      };
      
      console.log(`✅ Wallet topped up: +${this.formatUsdc(amountUSDC)} USDC`);
      
      return { success: true, receipt };
      
    } catch (error: any) {
      return { success: false, error: error.message };
    }
  }
  
  /**
   * Get payment statistics
   */
  getPaymentStats(): PaymentStats {
    let totalProcessed = BigInt(0);
    let totalTransactions = 0;
    let totalWallets = this.wallets.size;
    
    for (const tx of this.transactions.values()) {
      if (tx.status === "confirmed") {
        totalProcessed += tx.amount;
        totalTransactions++;
      }
    }
    
    return {
      totalProcessed: this.formatUsdc(totalProcessed),
      totalTransactions,
      totalWallets,
      averageTransactionSize: totalTransactions > 0
        ? this.formatUsdc(totalProcessed / BigInt(totalTransactions))
        : "0 USDC"
    };
  }
  
  /**
   * Generate x402 Payment Required header
   */
  generatePaymentHeader(
    walletAddress: string,
    amountUSDC: bigint,
    metadata?: Record<string, any>
  ): Record<string, string> {
    return {
      "Payment-Required": "true",
      "Payment-Protocol": "x402",
      "Payment-Amount-USDC": amountUSDC.toString(),
      "Payment-Receiver": this.config.paymentReceiverAddress || "",
      "Payment-Wallet": walletAddress,
      ...(metadata && { "Payment-Metadata": JSON.stringify(metadata) })
    };
  }
  
  /**
   * Verify x402 payment header from request
   */
  verifyPaymentHeader(headers: Record<string, string>): boolean {
    return (
      headers["payment-required"] === "true" &&
      headers["payment-protocol"] === "x402" &&
      headers["payment-amount-usdc"]
    );
  }
  
  /**
   * Get transaction history
   */
  getTransactionHistory(
    address: string,
    limit: number = 50
  ): PaymentTransaction[] {
    const normalizedAddress = ethers.getAddress(address);
    const wallet = this.wallets.get(normalizedAddress);
    
    if (!wallet) return [];
    
    return wallet.transactions.slice(-limit);
  }
  
  // ============================================================================
  // HELPER METHODS
  // ============================================================================
  
  /**
   * Convert USD to USDC (6 decimals)
   */
  private usdToUsdc(usd: number): bigint {
    // USDC has 6 decimals
    return BigInt(Math.ceil(usd * 1000000));
  }
  
  /**
   * Format USDC amount for display
   */
  private formatUsdc(amount: bigint): string {
    const decimals = 6;
    const divider = BigInt(10 ** decimals);
    const whole = amount / divider;
    const fractional = amount % divider;
    
    return `${whole}.${fractional.toString().padStart(decimals, "0")} USDC`;
  }
  
  /**
   * Generate unique transaction ID
   */
  private generateTransactionId(): string {
    return `tx_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
  }
  
  /**
   * Generate unique invoice ID
   */
  private generateInvoiceId(): string {
    return `inv_${Date.now().toString(36).toUpperCase()}_${Math.random().toString(36).slice(2, 9).toUpperCase()}`;
  }
}

/**
 * Payment Statistics
 */
export interface PaymentStats {
  totalProcessed: string;
  totalTransactions: number;
  totalWallets: number;
  averageTransactionSize: string;
}

export default X402PaymentHandler;
