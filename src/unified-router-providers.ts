/**
 * Unified Router - Provider Registry & Abstraction
 * Day 1: Base provider interface and registry system
 * 
 * Providers supported:
 * - Databricks
 * - AWS Bedrock
 * - OpenRouter
 * - Ollama (local)
 * - llama.cpp (local)
 * - LM Studio (local)
 * - Azure OpenAI
 * - Azure Anthropic
 */

import { 
  LLMRequest, 
  LLMResponse, 
  ProviderType, 
  ProviderConfig,
  ModelInfo,
  Tier
} from "./unified-router-types";

/**
 * Abstract base class for all providers
 */
export abstract class AbstractProviderClient {
  protected type: ProviderType;
  protected config: ProviderConfig;
  protected models: Map<string, ModelInfo> = new Map();
  protected isHealthy: boolean = true;
  
  constructor(type: ProviderType, config: ProviderConfig) {
    this.type = type;
    this.config = config;
  }
  
  /**
   * Initialize provider (load models, test connection)
   */
  abstract initialize(): Promise<void>;
  
  /**
   * Complete a request
   */
  abstract complete(request: LLMRequest): Promise<LLMResponse>;
  
  /**
   * Stream a request
   */
  abstract stream(request: LLMRequest): AsyncIterableIterator<LLMResponse>;
  
  /**
   * Get available models
   */
  abstract getModels(): ModelInfo[];
  
  /**
   * Get models for a specific tier
   */
  getModelsForTier(tier: Tier): ModelInfo[] {
    return Array.from(this.models.values()).filter(m => m.tier === tier);
  }
  
  /**
   * Find cheapest model for tier
   */
  getCheapestModel(tier: Tier): ModelInfo | null {
    const tierModels = this.getModelsForTier(tier);
    if (tierModels.length === 0) return null;
    
    return tierModels.reduce((prev, current) => 
      (prev.costInput + prev.costOutput) < (current.costInput + current.costOutput) 
        ? prev 
        : current
    );
  }
  
  /**
   * Health check
   */
  abstract healthCheck(): Promise<boolean>;
  
  /**
   * Format normalize: Convert provider response to standard LLMResponse
   */
  protected abstract normalizeResponse(
    raw: any,
    model: string
  ): LLMResponse;
  
  /**
   * Get provider type
   */
  getType(): ProviderType {
    return this.type;
  }
  
  /**
   * Is provider healthy?
   */
  getHealth(): boolean {
    return this.isHealthy;
  }
}

/**
 * Provider Registry - manages all providers
 */
export class ProviderRegistry {
  private providers: Map<ProviderType, AbstractProviderClient> = new Map();
  private configs: Map<ProviderType, ProviderConfig> = new Map();
  
  /**
   * Register a provider
   */
  registerProvider(
    type: ProviderType,
    client: AbstractProviderClient,
    config: ProviderConfig
  ): void {
    this.providers.set(type, client);
    this.configs.set(type, config);
  }
  
  /**
   * Get provider by type
   */
  getProvider(type: ProviderType): AbstractProviderClient | null {
    return this.providers.get(type) || null;
  }
  
  /**
   * Get all providers
   */
  getAllProviders(): AbstractProviderClient[] {
    return Array.from(this.providers.values());
  }
  
  /**
   * Get healthy providers only
   */
  getHealthyProviders(): AbstractProviderClient[] {
    return this.getAllProviders().filter(p => p.getHealth());
  }
  
  /**
   * Get models from all providers
   */
  getAllModels(): ModelInfo[] {
    return this.getAllProviders()
      .flatMap(p => p.getModels());
  }
  
  /**
   * Find model by ID across all providers
   */
  findModel(modelId: string): ModelInfo | null {
    for (const provider of this.getAllProviders()) {
      const model = provider.getModels().find(m => m.id === modelId);
      if (model) return model;
    }
    return null;
  }
  
  /**
   * Get cheapest model for tier across all providers
   */
  getCheapestModel(tier: Tier): ModelInfo | null {
    const allModels = this.getAllModels();
    const tierModels = allModels.filter(m => m.tier === tier);
    
    if (tierModels.length === 0) return null;
    
    return tierModels.reduce((prev, current) => 
      (prev.costInput + prev.costOutput) < (current.costInput + current.costOutput) 
        ? prev 
        : current
    );
  }
  
  /**
   * Get cheapest model for tier from specific provider
   */
  getCheapestModelFromProvider(
    tier: Tier,
    providerType: ProviderType
  ): ModelInfo | null {
    const provider = this.getProvider(providerType);
    if (!provider) return null;
    
    return provider.getCheapestModel(tier);
  }
  
  /**
   * Initialize all providers
   */
  async initializeAll(): Promise<void> {
    await Promise.all(
      this.getAllProviders().map(p => p.initialize())
    );
  }
  
  /**
   * Health check all providers
   */
  async healthCheckAll(): Promise<Record<ProviderType, boolean>> {
    const results: Record<string, boolean> = {};
    
    for (const [type, provider] of this.providers.entries()) {
      results[type] = await provider.healthCheck();
    }
    
    return results as Record<ProviderType, boolean>;
  }
}

// ============================================================================
// STUB IMPLEMENTATIONS (Day 1 - Minimal Working Examples)
// ============================================================================

/**
 * Ollama Provider (Local LLMs)
 */
export class OllamaProvider extends AbstractProviderClient {
  private endpoint: string;
  
  constructor(config: ProviderConfig) {
    super("ollama", config);
    this.endpoint = config.endpoint || "http://localhost:11434";
  }
  
  async initialize(): Promise<void> {
    // TODO: Pull default models, test connection
    this.models.set("qwen2.5-coder:latest", {
      id: "qwen2.5-coder:latest",
      provider: "ollama",
      name: "Qwen 2.5 Coder",
      tier: Tier.MEDIUM,
      costInput: 0,
      costOutput: 0,
      maxTokens: 32000,
      supports: {
        vision: false,
        tools: false,
        streaming: true,
        reasoning: false
      },
      avgLatency: 500,
      reliability: 0.99,
      available: true,
      lastHealthCheck: Date.now()
    });
  }
  
  async complete(request: LLMRequest): Promise<LLMResponse> {
    // TODO: Implement Ollama API call
    throw new Error("Not implemented - Day 1 stub");
  }
  
  async *stream(request: LLMRequest): AsyncIterableIterator<LLMResponse> {
    // TODO: Implement streaming
    throw new Error("Not implemented - Day 1 stub");
  }
  
  getModels(): ModelInfo[] {
    return Array.from(this.models.values());
  }
  
  async healthCheck(): Promise<boolean> {
    // TODO: Try to connect to Ollama
    this.isHealthy = true;
    return true;
  }
  
  protected normalizeResponse(raw: any, model: string): LLMResponse {
    // TODO: Convert Ollama response format
    throw new Error("Not implemented");
  }
}

/**
 * OpenRouter Provider
 */
export class OpenRouterProvider extends AbstractProviderClient {
  constructor(config: ProviderConfig) {
    super("openrouter", config);
  }
  
  async initialize(): Promise<void> {
    // TODO: Fetch model list from OpenRouter API
    this.models.set("gpt-4o", {
      id: "gpt-4o",
      provider: "openrouter",
      name: "GPT-4o",
      tier: Tier.COMPLEX,
      costInput: 2.5,
      costOutput: 10.0,
      maxTokens: 128000,
      supports: {
        vision: true,
        tools: true,
        streaming: true,
        reasoning: false
      },
      avgLatency: 300,
      reliability: 0.99,
      available: true,
      lastHealthCheck: Date.now()
    });
  }
  
  async complete(request: LLMRequest): Promise<LLMResponse> {
    throw new Error("Not implemented - Day 1 stub");
  }
  
  async *stream(request: LLMRequest): AsyncIterableIterator<LLMResponse> {
    throw new Error("Not implemented - Day 1 stub");
  }
  
  getModels(): ModelInfo[] {
    return Array.from(this.models.values());
  }
  
  async healthCheck(): Promise<boolean> {
    this.isHealthy = true;
    return true;
  }
  
  protected normalizeResponse(raw: any, model: string): LLMResponse {
    throw new Error("Not implemented");
  }
}

/**
 * AWS Bedrock Provider
 */
export class BedrockProvider extends AbstractProviderClient {
  constructor(config: ProviderConfig) {
    super("bedrock", config);
  }
  
  async initialize(): Promise<void> {
    // TODO: Connect to AWS and load models
  }
  
  async complete(request: LLMRequest): Promise<LLMResponse> {
    throw new Error("Not implemented - Day 1 stub");
  }
  
  async *stream(request: LLMRequest): AsyncIterableIterator<LLMResponse> {
    throw new Error("Not implemented - Day 1 stub");
  }
  
  getModels(): ModelInfo[] {
    return Array.from(this.models.values());
  }
  
  async healthCheck(): Promise<boolean> {
    this.isHealthy = true;
    return true;
  }
  
  protected normalizeResponse(raw: any, model: string): LLMResponse {
    throw new Error("Not implemented");
  }
}

export const providerRegistry = new ProviderRegistry();
