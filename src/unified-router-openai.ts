/**
 * OpenAI Provider for Unified Router
 * Supports GPT-4, GPT-4 Turbo, GPT-3.5 Turbo
 */

import {
  LLMProvider,
  ProviderConfig,
  RouterRequest,
  RouterResponse,
  TokenUsage,
  ProviderError
} from './unified-router-types';

export interface OpenAIConfig extends ProviderConfig {
  apiKey: string;
  baseURL?: string;
  organization?: string;
}

export class OpenAIProvider implements LLMProvider {
  name = 'openai';
  config: OpenAIConfig;
  private endpoint: string;

  constructor(config: OpenAIConfig) {
    if (!config.apiKey) {
      throw new Error('OpenAI API key required');
    }
    this.config = config;
    this.endpoint = config.baseURL || 'https://api.openai.com/v1';
  }

  async complete(request: RouterRequest): Promise<RouterResponse> {
    try {
      const model = this.selectModel(request);
      const messages = request.messages.map((msg: any) => ({
        role: msg.role,
        content: msg.content
      }));

      const response = await fetch(`${this.endpoint}/chat/completions`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${this.config.apiKey}`,
          ...(this.config.organization && { 'OpenAI-Organization': this.config.organization })
        },
        body: JSON.stringify({
          model,
          messages,
          temperature: request.temperature || 0.7,
          max_tokens: request.maxTokens || 2000,
          top_p: request.topP || 1,
          frequency_penalty: request.frequencyPenalty || 0,
          presence_penalty: request.presencePenalty || 0
        })
      });

      if (!response.ok) {
        const error = await response.json();
        throw new ProviderError(`OpenAI API error: ${error.error?.message}`, 'OPENAI_ERROR');
      }

      const data = await response.json();
      const choice = data.choices[0];

      return {
        message: choice.message.content,
        model: data.model,
        provider: 'openai',
        usage: {
          inputTokens: data.usage.prompt_tokens,
          outputTokens: data.usage.completion_tokens,
          totalTokens: data.usage.total_tokens
        },
        _costUSD: this.calculateCost(data.usage, data.model),
        _metadata: {
          finishReason: choice.finish_reason,
          createdAt: new Date(data.created * 1000)
        }
      };
    } catch (error: any) {
      if (error instanceof ProviderError) throw error;
      throw new ProviderError(
        `OpenAI provider error: ${error.message}`,
        'OPENAI_ERROR'
      );
    }
  }

  async getModels(): Promise<string[]> {
    return [
      'gpt-4-turbo-preview',
      'gpt-4',
      'gpt-3.5-turbo',
      'gpt-3.5-turbo-16k'
    ];
  }

  private selectModel(request: RouterRequest): string {
    if (request.model) return request.model;

    const complexity = this.estimateComplexity(request);

    switch (complexity) {
      case 'SIMPLE':
        return 'gpt-3.5-turbo';
      case 'MEDIUM':
        return 'gpt-3.5-turbo-16k';
      case 'COMPLEX':
        return 'gpt-4';
      case 'REASONING':
        return 'gpt-4-turbo-preview';
      default:
        return 'gpt-3.5-turbo';
    }
  }

  private estimateComplexity(request: RouterRequest): string {
    const inputLength = request.messages.reduce((sum, msg: any) => sum + msg.content.length, 0);

    if (inputLength > 10000) return 'REASONING';
    if (inputLength > 5000) return 'COMPLEX';
    if (inputLength > 1000) return 'MEDIUM';
    return 'SIMPLE';
  }

  private calculateCost(usage: TokenUsage, model: string): number {
    // Pricing as of 2024 (per 1M tokens)
    const pricing: Record<string, { input: number; output: number }> = {
      'gpt-4-turbo-preview': { input: 10, output: 30 },
      'gpt-4': { input: 30, output: 60 },
      'gpt-3.5-turbo': { input: 0.5, output: 1.5 },
      'gpt-3.5-turbo-16k': { input: 3, output: 4 }
    };

    const modelPricing = pricing[model] || pricing['gpt-3.5-turbo'];
    const inputCost = (usage.inputTokens / 1_000_000) * modelPricing.input;
    const outputCost = (usage.outputTokens / 1_000_000) * modelPricing.output;

    return inputCost + outputCost;
  }

  async health(): Promise<boolean> {
    try {
      const response = await fetch(`${this.endpoint}/models`, {
        headers: {
          'Authorization': `Bearer ${this.config.apiKey}`
        }
      });
      return response.ok;
    } catch {
      return false;
    }
  }
}

export default OpenAIProvider;
