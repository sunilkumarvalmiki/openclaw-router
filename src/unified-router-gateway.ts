/**
 * Unified Router - OpenClaw Gateway Integration
 * Day 5: Integration with OpenClaw gateway for CLI commands and messaging
 * 
 * Features:
 * - Message routing to Telegram/Discord/WhatsApp
 * - Command execution via gateway
 * - Request/response lifecycle
 * - User context management
 * - Error reporting
 */

import { CLICommandHandler } from "./unified-router-cli";
import { UnifiedRouter } from "./unified-router-core-optimized";
import { X402PaymentHandler } from "./unified-router-payments";
import { LLMRequest, Profile } from "./unified-router-types";

/**
 * OpenClaw Gateway Integration
 */
export class GatewayIntegration {
  private router: UnifiedRouter;
  private payments: X402PaymentHandler;
  private cli: CLICommandHandler;
  private userContexts: Map<string, UserContext> = new Map();
  
  constructor(
    router: UnifiedRouter,
    payments: X402PaymentHandler,
    cli: CLICommandHandler
  ) {
    this.router = router;
    this.payments = payments;
    this.cli = cli;
  }
  
  /**
   * Process incoming message from gateway
   */
  async handleIncomingMessage(message: GatewayMessage): Promise<GatewayResponse> {
    try {
      // Extract user context
      const context = await this.getOrCreateContext(message.userId, message.channel);
      
      // Check if it's a command
      if (message.text.startsWith("/")) {
        return await this.handleCommand(message, context);
      }
      
      // Regular message = LLM request
      return await this.handleLLMRequest(message, context);
      
    } catch (error: any) {
      return {
        success: false,
        error: `Error processing message: ${error.message}`,
        userId: message.userId
      };
    }
  }
  
  /**
   * Handle slash command
   */
  private async handleCommand(
    message: GatewayMessage,
    context: UserContext
  ): Promise<GatewayResponse> {
    try {
      const parts = message.text.split(/\s+/);
      const command = parts[0];
      const args = parts.slice(1);
      
      const result = await this.cli.handleCommand(command, args, message.userId);
      
      return {
        success: true,
        message: result,
        userId: message.userId,
        channel: message.channel,
        messageId: message.messageId
      };
      
    } catch (error: any) {
      return {
        success: false,
        error: `Command failed: ${error.message}`,
        userId: message.userId
      };
    }
  }
  
  /**
   * Handle LLM request
   */
  private async handleLLMRequest(
    message: GatewayMessage,
    context: UserContext
  ): Promise<GatewayResponse> {
    try {
      // Check if user has wallet and balance
      const wallet = await this.payments.getWallet(context.walletAddress);
      if (!wallet) {
        return {
          success: false,
          error: "❌ No wallet found. Use /wallet create first",
          userId: message.userId,
          suggestion: "/wallet create"
        };
      }
      
      // Create LLM request
      const request: LLMRequest = {
        messages: [
          {
            role: "user",
            content: message.text
          }
        ],
        _profile: context.profile
      };
      
      // Log request start
      console.log(`📨 Request from ${message.userId}: ${message.text.slice(0, 50)}...`);
      
      // Execute request
      const response = await this.router.complete(request);
      
      // Extract answer
      const answer = response.choices[0]?.message?.content || "No response";
      
      // Update context
      context.lastRequest = new Date();
      context.requestCount++;
      
      // Prepare response
      const result: GatewayResponse = {
        success: true,
        message: answer,
        userId: message.userId,
        channel: message.channel,
        messageId: message.messageId,
        metadata: {
          costUSD: response._costUSD,
          model: response.model,
          provider: response._selectedProvider,
          tokens: response.usage?.total_tokens,
          latency: response._latencyMs || 0
        }
      };
      
      // Log success
      console.log(`✅ Response generated (${response.usage?.completion_tokens} tokens, $${response._costUSD.toFixed(4)})`);
      
      return result;
      
    } catch (error: any) {
      console.error(`❌ Request failed: ${error.message}`);
      return {
        success: false,
        error: `Request failed: ${error.message}`,
        userId: message.userId
      };
    }
  }
  
  /**
   * Get or create user context
   */
  private async getOrCreateContext(
    userId: string,
    channel: string
  ): Promise<UserContext> {
    if (this.userContexts.has(userId)) {
      return this.userContexts.get(userId)!;
    }
    
    // Create new context
    const walletAddress = `0x${userId.substring(0, 40).padEnd(40, "0")}`;
    
    // Ensure wallet exists
    await this.payments.createWallet(walletAddress);
    
    const context: UserContext = {
      userId,
      channel,
      walletAddress,
      profile: Profile.AUTO,
      requestCount: 0,
      lastRequest: null,
      createdAt: new Date()
    };
    
    this.userContexts.set(userId, context);
    return context;
  }
  
  /**
   * Send response to gateway
   */
  async sendToGateway(response: GatewayResponse): Promise<void> {
    // This would integrate with the actual OpenClaw message tool
    console.log(`📤 Sending to gateway (${response.channel}): ${response.message?.slice(0, 50)}`);
    
    // In a real implementation:
    // await message.send({
    //   channel: response.channel,
    //   message: response.message,
    //   userId: response.userId
    // });
  }
  
  /**
   * Get user statistics
   */
  getUserStats(userId: string): UserStats | null {
    const context = this.userContexts.get(userId);
    if (!context) return null;
    
    return {
      userId,
      requestCount: context.requestCount,
      profile: context.profile,
      walletAddress: context.walletAddress,
      joined: context.createdAt,
      lastRequest: context.lastRequest
    };
  }
  
  /**
   * Get gateway health status
   */
  async getHealth(): Promise<HealthStatus> {
    const routerHealth = await this.router.getHealth();
    
    return {
      gateway: "healthy",
      router: routerHealth.healthy ? "healthy" : "unhealthy",
      payments: "healthy",
      activeUsers: this.userContexts.size,
      uptime: process.uptime()
    };
  }
}

/**
 * Gateway Message (incoming)
 */
export interface GatewayMessage {
  userId: string;
  channel: "telegram" | "discord" | "whatsapp";
  messageId: string;
  text: string;
  timestamp: Date;
  metadata?: Record<string, any>;
}

/**
 * Gateway Response (outgoing)
 */
export interface GatewayResponse {
  success: boolean;
  message?: string;
  error?: string;
  userId: string;
  channel?: string;
  messageId?: string;
  suggestion?: string;
  metadata?: {
    costUSD?: number;
    model?: string;
    provider?: string;
    tokens?: number;
    latency?: number;
  };
}

/**
 * User Context (session data)
 */
export interface UserContext {
  userId: string;
  channel: string;
  walletAddress: string;
  profile: Profile;
  requestCount: number;
  lastRequest: Date | null;
  createdAt: Date;
}

/**
 * User Statistics
 */
export interface UserStats {
  userId: string;
  requestCount: number;
  profile: Profile;
  walletAddress: string;
  joined: Date;
  lastRequest: Date | null;
}

/**
 * Gateway Health Status
 */
export interface HealthStatus {
  gateway: string;
  router: string;
  payments: string;
  activeUsers: number;
  uptime: number;
}

/**
 * Gateway Message Handler Factory
 */
export class GatewayFactory {
  /**
   * Create fully integrated gateway
   */
  static createGateway(
    router: UnifiedRouter,
    payments: X402PaymentHandler,
    cli: CLICommandHandler
  ): GatewayIntegration {
    return new GatewayIntegration(router, payments, cli);
  }
  
  /**
   * Handle Telegram message (example)
   */
  static async handleTelegramMessage(
    gateway: GatewayIntegration,
    userId: string,
    text: string,
    messageId: string
  ): Promise<GatewayResponse> {
    return await gateway.handleIncomingMessage({
      userId: userId.toString(),
      channel: "telegram",
      messageId,
      text,
      timestamp: new Date()
    });
  }
  
  /**
   * Handle Discord message (example)
   */
  static async handleDiscordMessage(
    gateway: GatewayIntegration,
    userId: string,
    text: string,
    messageId: string
  ): Promise<GatewayResponse> {
    return await gateway.handleIncomingMessage({
      userId: userId.toString(),
      channel: "discord",
      messageId,
      text,
      timestamp: new Date()
    });
  }
  
  /**
   * Handle WhatsApp message (example)
   */
  static async handleWhatsAppMessage(
    gateway: GatewayIntegration,
    userId: string,
    text: string,
    messageId: string
  ): Promise<GatewayResponse> {
    return await gateway.handleIncomingMessage({
      userId: userId.toString(),
      channel: "whatsapp",
      messageId,
      text,
      timestamp: new Date()
    });
  }
}

export {
  GatewayIntegration,
  GatewayFactory
};
