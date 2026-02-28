/**
 * Unified Router - Main Orchestrator
 * Day 1: Core router class that ties everything together
 */

import {
  LLMRequest,
  LLMResponse,
  RouterConfig,
  Profile,
  Tier,
  SelectionCriteria,
  SelectedModel,
  RouterMetrics,
  HealthStatus
} from "./unified-router-types";

import { ScoringEngine } from "./unified-router-scorer";
import { ProviderRegistry } from "./unified-router-providers";

export class UnifiedRouter {
  private config: RouterConfig;
  private scorer: ScoringEngine;
  private providers: ProviderRegistry;
  private metrics: RouterMetrics;
  
  constructor(config: RouterConfig, providers: ProviderRegistry) {
    this.config = config;
    this.scorer = new ScoringEngine();
    this.providers = providers;
    this.metrics = this.initializeMetrics();
  }
  
  private initializeMetrics(): RouterMetrics {
    return {
      totalRequests: 0,
      successfulRequests: 0,
      failedRequests: 0,
      totalCostUSD: 0,
      averageLatency: 0,
      cacheHitRate: 0,
      profileDistribution: {
        [Profile.ECO]: 0,
        [Profile.AUTO]: 0,
        [Profile.PREMIUM]: 0,
        [Profile.FREE]: 0,
      },
      tierDistribution: {
        [Tier.SIMPLE]: 0,
        [Tier.MEDIUM]: 0,
        [Tier.COMPLEX]: 0,
        [Tier.REASONING]: 0,
      },
      providerDistribution: {}
    };
  }
  
  /**
   * Main entry point: Complete a request
   */
  async complete(request: LLMRequest): Promise<LLMResponse> {
    const startTime = Date.now();
    
    try {
      this.metrics.totalRequests++;
      
      // Step 1: Score the request (15-dimensional scoring)
      const profile = request._profile || this.config.profile;
      const tier = request._tier || this.scorer.score(request).tier;
      
      // Step 2: Update metrics
      this.metrics.profileDistribution[profile]++;
      this.metrics.tierDistribution[tier]++;
      
      // Step 3: Select the best model
      const selection = this.selectModel({
        tier,
        profile,
        providers: request._provider ? [request._provider as any] : undefined
      });
      
      if (!selection) {
        throw new Error(`No suitable model found for tier: ${tier}`);
      }
      
      // Step 4: Get provider and dispatch
      const provider = this.providers.getProvider(selection.provider);
      if (!provider) {
        throw new Error(`Provider not found: ${selection.provider}`);
      }
      
      // Step 5: Execute request
      const response = await provider.complete({
        ...request,
        model: selection.model
      });
      
      // Step 6: Update metrics
      this.metrics.successfulRequests++;
      this.metrics.totalCostUSD += response._costUSD || 0;
      const latency = Date.now() - startTime;
      this.metrics.averageLatency = 
        (this.metrics.averageLatency * (this.metrics.successfulRequests - 1) + latency) /
        this.metrics.successfulRequests;
      
      // Enrich response with router metadata
      response._selectedModel = selection.model;
      response._selectedProvider = selection.provider;
      response._tier = tier;
      response._profile = profile;
      
      return response;
      
    } catch (error) {
      this.metrics.failedRequests++;
      throw error;
    }
  }
  
  /**
   * Stream a request (returns async iterator)
   */
  async *stream(request: LLMRequest): AsyncIterableIterator<LLMResponse> {
    const profile = request._profile || this.config.profile;
    const tier = request._tier || this.scorer.score(request).tier;
    
    const selection = this.selectModel({ tier, profile });
    if (!selection) {
      throw new Error(`No suitable model found for tier: ${tier}`);
    }
    
    const provider = this.providers.getProvider(selection.provider);
    if (!provider) {
      throw new Error(`Provider not found: ${selection.provider}`);
    }
    
    yield* provider.stream({
      ...request,
      model: selection.model
    });
  }
  
  /**
   * Select the best model based on tier and profile
   */
  private selectModel(criteria: SelectionCriteria): SelectedModel | null {
    const { tier, profile, budget, providers, excludeProviders } = criteria;
    
    if (profile === Profile.FREE) {
      // Only use free models (e.g., gpt-oss-120b, Ollama)
      return this.selectFreeModel(tier);
    }
    
    if (profile === Profile.ECO) {
      // Select cheapest capable model
      return this.selectCheapestModel(tier, budget, providers, excludeProviders);
    }
    
    if (profile === Profile.AUTO) {
      // Balanced: use mid-range models
      return this.selectBalancedModel(tier, budget, providers);
    }
    
    if (profile === Profile.PREMIUM) {
      // Best quality, no cost consideration
      return this.selectPremiumModel(tier);
    }
    
    return null;
  }
  
  private selectFreeModel(tier: Tier): SelectedModel | null {
    // TODO: Implement free model selection
    return {
      model: "gpt-oss-120b",
      provider: "openrouter",
      tier,
      estimatedCost: 0,
      reason: "Free tier model"
    };
  }
  
  private selectCheapestModel(
    tier: Tier,
    budget?: number,
    providers?: any[],
    excludeProviders?: any[]
  ): SelectedModel | null {
    // TODO: Implement cheapest model selection
    return {
      model: "gemini-2.5-flash-lite",
      provider: "openrouter",
      tier,
      estimatedCost: 0.1,
      reason: "Cheapest capable model"
    };
  }
  
  private selectBalancedModel(
    tier: Tier,
    budget?: number,
    providers?: any[]
  ): SelectedModel | null {
    // TODO: Implement balanced selection
    return {
      model: "grok-code-fast",
      provider: "openrouter",
      tier,
      estimatedCost: 0.3,
      reason: "Balanced cost/performance"
    };
  }
  
  private selectPremiumModel(tier: Tier): SelectedModel | null {
    // TODO: Implement premium selection
    return {
      model: "claude-opus-4.5",
      provider: "bedrock",
      tier,
      estimatedCost: 25,
      reason: "Best quality available"
    };
  }
  
  /**
   * Get current metrics
   */
  getMetrics(): RouterMetrics {
    return { ...this.metrics };
  }
  
  /**
   * Get health status
   */
  async getHealth(): Promise<HealthStatus> {
    const providerHealth = await this.providers.healthCheckAll();
    
    const status: "healthy" | "degraded" | "unhealthy" =
      Object.values(providerHealth).every(h => h) ? "healthy" :
      Object.values(providerHealth).some(h => h) ? "degraded" :
      "unhealthy";
    
    return {
      status,
      providers: providerHealth,
      metrics: this.getMetrics(),
      lastCheck: Date.now()
    };
  }
  
  /**
   * Initialize router (load providers, test connections)
   */
  async initialize(): Promise<void> {
    await this.providers.initializeAll();
  }
}

export const createRouter = (
  config: RouterConfig,
  providers: ProviderRegistry
): UnifiedRouter => {
  return new UnifiedRouter(config, providers);
};
