/**
 * OpenClaw Router Integration
 * Bridges Unified Router with OpenClaw gateway
 */

import { GatewayIntegration } from '../unified-router-gateway';
import { UnifiedRouter } from '../unified-router-core-optimized';
import { X402PaymentHandler } from '../unified-router-payments';
import { CLICommandHandler } from '../unified-router-cli';
import { ProviderRegistry } from '../unified-router-providers';

/**
 * OpenClaw Router Handler
 */
export class OpenClawRouter {
  private gateway: GatewayIntegration;
  private router: UnifiedRouter;
  private payments: X402PaymentHandler;
  private cli: CLICommandHandler;
  
  constructor() {
    const registry = new ProviderRegistry();
    this.router = new UnifiedRouter(
      {
        profile: 'AUTO',
        providers: [],
        optimization: {
          enableCache: true,
          enableDedup: true,
          enableCompress: true,
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
    
    this.payments = new X402PaymentHandler({ enabled: true });
    this.cli = new CLICommandHandler(this.router, this.payments);
    this.gateway = new GatewayIntegration(this.router, this.payments, this.cli);
  }
  
  /**
   * Handle Telegram message via OpenClaw
   */
  async handleTelegramMessage(message: {
    text: string;
    from: { id: number; username: string };
    message_id: number;
  }): Promise<string> {
    const response = await this.gateway.handleIncomingMessage({
      userId: message.from.id.toString(),
      channel: 'telegram',
      messageId: message.message_id.toString(),
      text: message.text,
      timestamp: new Date()
    });
    
    return response.message || response.error || 'No response';
  }
  
  /**
   * Handle Discord message via OpenClaw
   */
  async handleDiscordMessage(message: {
    content: string;
    author: { id: string; username: string };
    id: string;
  }): Promise<string> {
    const response = await this.gateway.handleIncomingMessage({
      userId: message.author.id,
      channel: 'discord',
      messageId: message.id,
      text: message.content,
      timestamp: new Date()
    });
    
    return response.message || response.error || 'No response';
  }
  
  /**
   * Get router health
   */
  async getHealth() {
    return await this.gateway.getHealth();
  }
  
  /**
   * Get user statistics
   */
  getUserStats(userId: string) {
    return this.gateway.getUserStats(userId);
  }
  
  /**
   * Get router metrics
   */
  getMetrics() {
    return this.router.getMetrics();
  }
}

export default new OpenClawRouter();
