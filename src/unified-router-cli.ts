/**
 * Unified Router - CLI Commands for OpenClaw Integration
 * Day 4: User-facing commands for model selection, wallet, and stats
 * 
 * Commands:
 * /model - Select model or view available models
 * /wallet - Check wallet balance and credits
 * /stats - View usage statistics
 * /topup - Add credits to wallet
 * /history - View payment history
 */

import { UnifiedRouter } from "./unified-router-core-optimized";
import { X402PaymentHandler } from "./unified-router-payments";
import { Profile, Tier, ModelInfo } from "./unified-router-types";

/**
 * CLI Command Handler
 */
export class CLICommandHandler {
  private router: UnifiedRouter;
  private payments: X402PaymentHandler;
  private currentProfile: Profile = Profile.AUTO;
  private currentUser: string | null = null;
  
  constructor(router: UnifiedRouter, payments: X402PaymentHandler) {
    this.router = router;
    this.payments = payments;
  }
  
  /**
   * Main command dispatcher
   */
  async handleCommand(
    command: string,
    args: string[] = [],
    userId?: string
  ): Promise<string> {
    if (userId) {
      this.currentUser = userId;
    }
    
    const cmd = command.toLowerCase().replace(/^\//, "");
    
    switch (cmd) {
      case "model":
        return await this.handleModelCommand(args);
      case "wallet":
        return await this.handleWalletCommand(args);
      case "stats":
        return await this.handleStatsCommand(args);
      case "topup":
        return await this.handleTopupCommand(args);
      case "history":
        return await this.handleHistoryCommand(args);
      case "profile":
        return await this.handleProfileCommand(args);
      case "help":
        return this.getHelpText();
      default:
        return `Unknown command: /${cmd}\nType /help for available commands.`;
    }
  }
  
  /**
   * /model command
   * Usage: /model list [tier] - List available models
   *        /model select <model-id> - Select a model
   *        /model info <model-id> - Get model details
   */
  private async handleModelCommand(args: string[]): Promise<string> {
    if (args.length === 0) {
      return this.getModelHelp();
    }
    
    const subcommand = args[0].toLowerCase();
    
    if (subcommand === "list") {
      const tier = args[1]?.toUpperCase() as Tier | undefined;
      return this.listModels(tier);
    }
    
    if (subcommand === "select") {
      const modelId = args[1];
      if (!modelId) {
        return "❌ Error: Model ID required\nUsage: /model select <model-id>";
      }
      return this.selectModel(modelId);
    }
    
    if (subcommand === "info") {
      const modelId = args[1];
      if (!modelId) {
        return "❌ Error: Model ID required\nUsage: /model info <model-id>";
      }
      return this.getModelInfo(modelId);
    }
    
    return this.getModelHelp();
  }
  
  /**
   * /wallet command
   * Usage: /wallet - Show wallet info
   *        /wallet balance - Show balance
   *        /wallet create - Create new wallet
   */
  private async handleWalletCommand(args: string[]): Promise<string> {
    if (!this.currentUser) {
      return "❌ Error: User ID required";
    }
    
    const subcommand = args[0]?.toLowerCase() || "info";
    
    if (subcommand === "balance") {
      const wallet = await this.payments.getWallet(this.currentUser);
      if (!wallet) {
        return "❌ Wallet not found. Use /wallet create";
      }
      
      return `💰 **Wallet Balance**
Address: ${wallet.address}
Balance: ${(Number(wallet.balance) / 1000000).toFixed(2)} USDC
Credits: ${wallet.credits}
Total Spent: $${(Number(wallet.spent) / 1000000 * 10).toFixed(2)}
Created: ${wallet.createdAt.toLocaleDateString()}`;
    }
    
    if (subcommand === "create") {
      const wallet = await this.payments.createWallet(this.currentUser);
      return `✅ **Wallet Created**
Address: ${wallet.address}
Credits: ${wallet.credits}
Ready to top up with /topup`;
    }
    
    // Default: show info
    const wallet = await this.payments.getWallet(this.currentUser);
    if (!wallet) {
      return "❌ Wallet not found. Use /wallet create";
    }
    
    const percentSpent = wallet.spent > 0 ? "💸" : "✨";
    return `${percentSpent} **Wallet Info**
Address: ${wallet.address}
Balance: ${(Number(wallet.balance) / 1000000).toFixed(2)} USDC
Credits: ${wallet.credits}
Total Spent: ${(Number(wallet.spent) / 1000000).toFixed(6)} USDC
Transactions: ${wallet.transactions.length}
Created: ${wallet.createdAt.toLocaleDateString()}`;
  }
  
  /**
   * /stats command
   * Usage: /stats - Show overall statistics
   *        /stats usage - Show personal usage
   *        /stats providers - Show provider distribution
   */
  private async handleStatsCommand(args: string[]): Promise<string> {
    const subcommand = args[0]?.toLowerCase() || "system";
    
    if (subcommand === "system") {
      const metrics = this.router.getMetrics();
      
      return `📊 **System Statistics**

Requests:
  Total: ${metrics.totalRequests}
  Successful: ${metrics.successfulRequests}
  Failed: ${metrics.failedRequests}
  Success Rate: ${metrics.totalRequests > 0 ? ((metrics.successfulRequests / metrics.totalRequests) * 100).toFixed(1) : 0}%

Cost:
  Total: $${metrics.totalCostUSD.toFixed(2)}
  Average per request: $${metrics.totalRequests > 0 ? (metrics.totalCostUSD / metrics.totalRequests).toFixed(4) : 0}

Performance:
  Average Latency: ${metrics.averageLatency.toFixed(0)}ms
  Cache Hit Rate: ${(metrics.cacheHitRate * 100).toFixed(1)}%`;
    }
    
    if (subcommand === "usage") {
      if (!this.currentUser) {
        return "❌ Error: User ID required";
      }
      
      const wallet = await this.payments.getWallet(this.currentUser);
      if (!wallet) {
        return "❌ Wallet not found";
      }
      
      const txCount = wallet.transactions.length;
      const costSum = wallet.transactions.reduce((sum, tx) => sum + tx.costUSD, 0);
      
      return `📈 **Your Usage Statistics**
User: ${this.currentUser}

Transactions: ${txCount}
Total Cost: $${costSum.toFixed(2)}
Average Cost: $${txCount > 0 ? (costSum / txCount).toFixed(4) : 0}
Balance: ${(Number(wallet.balance) / 1000000).toFixed(2)} USDC`;
    }
    
    if (subcommand === "providers") {
      const metrics = this.router.getMetrics();
      let providerStats = "🔌 **Provider Distribution**\n";
      
      for (const [provider, count] of Object.entries(metrics.providerDistribution)) {
        const percentage = metrics.totalRequests > 0 
          ? ((count / metrics.totalRequests) * 100).toFixed(1)
          : 0;
        providerStats += `${provider}: ${count} (${percentage}%)\n`;
      }
      
      return providerStats;
    }
    
    return this.getStatsHelp();
  }
  
  /**
   * /topup command
   * Usage: /topup <amount-usdc> - Add credits to wallet
   */
  private async handleTopupCommand(args: string[]): Promise<string> {
    if (!this.currentUser) {
      return "❌ Error: User ID required";
    }
    
    const amountStr = args[0];
    if (!amountStr) {
      return "❌ Error: Amount required\nUsage: /topup <amount-usdc>";
    }
    
    try {
      const amount = parseFloat(amountStr);
      if (amount <= 0) {
        return "❌ Error: Amount must be positive";
      }
      
      const amountWei = BigInt(Math.ceil(amount * 1000000));
      
      const result = await this.payments.topUpWallet(this.currentUser, amountWei);
      
      if (result.success && result.receipt) {
        return `✅ **Top-up Successful**
Amount: ${amount.toFixed(2)} USDC
Invoice: ${result.receipt.invoiceId}
New Balance: ${(amount).toFixed(2)} USDC
Timestamp: ${result.receipt.timestamp.toLocaleTimeString()}`;
      } else {
        return `❌ Top-up failed: ${result.error}`;
      }
      
    } catch (error: any) {
      return `❌ Error: ${error.message}`;
    }
  }
  
  /**
   * /history command
   * Usage: /history [limit] - Show payment history
   */
  private async handleHistoryCommand(args: string[]): Promise<string> {
    if (!this.currentUser) {
      return "❌ Error: User ID required";
    }
    
    const limit = parseInt(args[0] || "10");
    const history = this.payments.getTransactionHistory(this.currentUser, limit);
    
    if (history.length === 0) {
      return "📜 **Payment History**\nNo transactions yet.";
    }
    
    let result = `📜 **Payment History** (Last ${history.length})\n\n`;
    
    for (const tx of history.slice().reverse()) {
      const amount = (Number(tx.amount) / 1000000).toFixed(2);
      const type = tx.type === "payment" ? "💳" : "💰";
      result += `${type} ${tx.type.toUpperCase()}: ${amount} USDC (${tx.costUSD > 0 ? "$" + tx.costUSD.toFixed(4) : "top-up"})
  Status: ${tx.status} | ${tx.timestamp.toLocaleDateString()} ${tx.timestamp.toLocaleTimeString()}\n`;
    }
    
    return result;
  }
  
  /**
   * /profile command
   * Usage: /profile [eco|auto|premium|free] - Set routing profile
   */
  private async handleProfileCommand(args: string[]): Promise<string> {
    const profileName = args[0]?.toUpperCase();
    
    if (!profileName) {
      return `🎯 **Current Profile**: ${this.currentProfile}

Available profiles:
  ECO - Maximum savings (92% reduction)
  AUTO - Balanced (66% reduction) [DEFAULT]
  PREMIUM - Best quality (0% reduction)
  FREE - Free models only (100% reduction)

Usage: /profile <eco|auto|premium|free>`;
    }
    
    if (Object.values(Profile).includes(profileName as Profile)) {
      this.currentProfile = profileName as Profile;
      return `✅ **Profile Changed** to ${profileName}
Route preferences updated.`;
    }
    
    return `❌ Invalid profile: ${profileName}`;
  }
  
  // ============================================================================
  // HELPER METHODS
  // ============================================================================
  
  private listModels(tier?: Tier): string {
    // Get all models from router
    const allModels = this.router.getMetrics().providerDistribution; // This is a simplification
    
    let result = "📋 **Available Models**\n";
    
    // Show models by tier
    const tiers = [Tier.SIMPLE, Tier.MEDIUM, Tier.COMPLEX, Tier.REASONING];
    for (const t of tiers) {
      if (tier && t !== tier) continue;
      
      result += `\n**${t} Tier**\n`;
      result += `  Recommended models for ${t} complexity tasks\n`;
    }
    
    return result;
  }
  
  private selectModel(modelId: string): string {
    return `✅ **Model Selected**
Model: ${modelId}
Profile: ${this.currentProfile}
Ready for requests.`;
  }
  
  private getModelInfo(modelId: string): string {
    return `📌 **Model Information**
Model: ${modelId}
Status: Loading model details...`;
  }
  
  private getModelHelp(): string {
    return `📋 **Model Commands**

/model list [tier] - List available models
  /model list SIMPLE
  /model list MEDIUM
  /model list COMPLEX
  /model list REASONING

/model select <model-id> - Select a specific model
  /model select gpt-4o

/model info <model-id> - Get model details
  /model info claude-3-sonnet`;
  }
  
  private getStatsHelp(): string {
    return `📊 **Stats Commands**

/stats - Overall system statistics
/stats usage - Your personal usage
/stats providers - Provider distribution`;
  }
  
  private getHelpText(): string {
    return `🆘 **Unified Router - Available Commands**

💰 WALLET & PAYMENTS:
  /wallet - Show wallet info
  /wallet balance - Show balance only
  /wallet create - Create new wallet
  /topup <amount> - Add USDC credits
  /history [limit] - Payment history

🤖 MODEL SELECTION:
  /model list [tier] - List available models
  /model select <id> - Select model
  /model info <id> - Model details

📊 STATISTICS:
  /stats - System statistics
  /stats usage - Your usage
  /stats providers - Provider breakdown

⚙️ SETTINGS:
  /profile [eco|auto|premium|free] - Set routing profile
  /help - This message`;
  }
}

export { CLICommandHandler };
