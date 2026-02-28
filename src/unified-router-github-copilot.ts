/**
 * GitHub Copilot Provider for Unified Router
 * Uses GitHub Copilot API (GitHub Models API)
 */

import {
  LLMProvider,
  ProviderConfig,
  RouterRequest,
  RouterResponse,
  TokenUsage,
  ProviderError
} from './unified-router-types';

export interface GitHubCopilotConfig extends ProviderConfig {
  token: string;
  baseURL?: string;
}

export class GitHubCopilotProvider implements LLMProvider {
  name = 'github-copilot';
  config: GitHubCopilotConfig;
  private endpoint: string;

  constructor(config: GitHubCopilotConfig) {
    if (!config.token) {
      throw new Error('GitHub token required for Copilot provider');
    }
    this.config = config;
    this.endpoint = config.baseURL || 'https://models.inference.ai.github.com';
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
          'Authorization': `Bearer ${this.config.token}`,
          'User-Agent': 'openclaw-router/1.0.0'
        },
        body: JSON.stringify({
          model,
          messages,
          temperature: request.temperature || 0.7,
          max_tokens: request.maxTokens || 2000,
          top_p: request.topP || 1
        })
      });

      if (!response.ok) {
        const error = await response.json();
        throw new ProviderError(
          `GitHub Copilot API error: ${error.message || response.statusText}`,
          'GITHUB_COPILOT_ERROR'
        );
      }

      const data = await response.json();
      const choice = data.choices[0];

      return {
        message: choice.message.content,
        model: data.model,
        provider: 'github-copilot',
        usage: {
          inputTokens: data.usage?.prompt_tokens || 0,
          outputTokens: data.usage?.completion_tokens || 0,
          totalTokens: data.usage?.total_tokens || 0
        },
        _costUSD: 0, // GitHub Copilot enterprise models are often free for enterprise
        _metadata: {
          finishReason: choice.finish_reason,
          model: data.model
        }
      };
    } catch (error: any) {
      if (error instanceof ProviderError) throw error;
      throw new ProviderError(
        `GitHub Copilot provider error: ${error.message}`,
        'GITHUB_COPILOT_ERROR'
      );
    }
  }

  async getModels(): Promise<string[]> {
    // GitHub Models API available models
    return [
      'gpt-4o',
      'gpt-4-turbo',
      'gpt-35-turbo',
      'claude-3-5-sonnet',
      'mistral-large',
      'meta-llama-3-1-70b-instruct',
      'phi-3-mini-4k-instruct'
    ];
  }

  private selectModel(request: RouterRequest): string {
    if (request.model) return request.model;

    const complexity = this.estimateComplexity(request);

    switch (complexity) {
      case 'SIMPLE':
        return 'gpt-35-turbo';
      case 'MEDIUM':
        return 'gpt-4o';
      case 'COMPLEX':
        return 'gpt-4-turbo';
      case 'REASONING':
        return 'claude-3-5-sonnet';
      default:
        return 'gpt-4o';
    }
  }

  private estimateComplexity(request: RouterRequest): string {
    const inputLength = request.messages.reduce((sum, msg: any) => sum + msg.content.length, 0);

    if (inputLength > 10000) return 'REASONING';
    if (inputLength > 5000) return 'COMPLEX';
    if (inputLength > 1000) return 'MEDIUM';
    return 'SIMPLE';
  }

  async health(): Promise<boolean> {
    try {
      const response = await fetch(`${this.endpoint}/models`, {
        method: 'GET',
        headers: {
          'Authorization': `Bearer ${this.config.token}`
        }
      });
      return response.ok;
    } catch {
      return false;
    }
  }
}

export default GitHubCopilotProvider;
