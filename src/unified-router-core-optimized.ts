/**
 * Unified Router - Main Orchestrator with Token Optimization
 * Day 3 Update: Integrated semantic caching, deduplication, compression
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
import {
  TokenOptimizer,
  RequestDeduplicator,
  OptimizationResult
} from "./unified-router-optimizer";

export class UnifiedRouter {
  private config: RouterConfig;
  private scorer: ScoringEngine;
  private providers: ProviderRegistry;
  private optimizer: TokenOptimizer;
  private dedup: RequestDeduplicator;
  private metrics: RouterMetrics;
  private inFlightRequests: Map<string, Promise<LLMResponse>>;
  
  constructor(config: RouterConfig, providers: ProviderRegistry) {
    this.config = config;
    this.scorer = new ScoringEngine();
    this.providers = providers;
    this.optimizer = new TokenOptimizer(
      10000,  // cache size
      3600000, // 1 hour TTL
      1000     // 1 second dedup window
    );
    this.dedup = new RequestDeduplicator(1000);
    this.metrics = this.initializeMetrics();
    this.inFlightRequests = new Map();
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
      providerDistribution: {},
      optimizationStats: {
        cacheHits: 0,
        dedupHits: 0,
        tokensSaved: 0,
        averageCompressionPercent: 0
      }
    };
  }
  
  /**
   * Main entry point: Complete a request (with optimization)
   */
  async complete(request: LLMRequest): Promise<LLMResponse> {
    const startTime = Date.now();
    
    try {
      this.metrics.totalRequests++;
      
      // === OPTIMIZATION STEP 1: Check for deduplication ===
      const pendingPromise = this.dedup.getIfPending(request);
      if (pendingPromise) {
        // Another identical request is in flight, wait for it
        const response = await pendingPromise;
        this.metrics.optimizationStats!.dedupHits!++;
        return response;
      }
      
      // === OPTIMIZATION STEP 2: Optimize the request ===
      const optimization = this.optimizer.optimizeRequest(request);
      
      if (optimization.cachedResponse) {
        // Cache hit! Return immediately
        this.metrics.optimizationStats!.cacheHits!++;
        this.metrics.optimizationStats!.tokensSaved! += optimization.metadata.savings;
        return optimization.cachedResponse;
      }
      
      // Use optimized request going forward
      const optimizedRequest = optimization.request;
      
      // === ORIGINAL ROUTING LOGIC ===
      const profile = optimizedRequest._profile || this.config.profile;
      const tier = optimizedRequest._tier || this.scorer.score(optimizedRequest).tier;
      
      // Update metrics
      this.metrics.profileDistribution[profile]++;
      this.metrics.tierDistribution[tier]++;
      
      // Select the best model
      const selection = this.selectModel({
        tier,
        profile,
        providers: optimizedRequest._provider ? [optimizedRequest._provider as any] : undefined
      });
      
      if (!selection) {
        throw new Error(`No suitable model found for tier: ${tier}`);
      }
      
      // Get provider and dispatch
      const provider = this.providers.getProvider(selection.provider);
      if (!provider) {
        throw new Error(`Provider not found: ${selection.provider}`);
      }
      
      // Register in-flight request for deduplication
      const responsePromise = provider.complete({
        ...optimizedRequest,
        model: selection.model
      });
      
      this.dedup.register(request, responsePromise);
      
      // Execute request
      const response = await responsePromise;
      
      // Update metrics
      this.metrics.successfulRequests++;
      this.metrics.totalCostUSD += response._costUSD || 0;
      
      const latency = Date.now() - startTime;
      this.metrics.averageLatency = (
        (this.metrics.averageLatency * (this.metrics.successfulRequests - 1) + latency) /
        this.metrics.successfulRequests
      );
      
      // Update provider distribution
      const provider_key = selection.provider;
      this.metrics.providerDistribution[provider_key] = 
        (this.metrics.providerDistribution[provider_key] || 0) + 1;
      
      // === OPTIMIZATION STEP 3: Cache the response ===
      this.optimizer.cacheResponse(request, response);
      
      // Add optimization metadata to response
      response._optimizationMetadata = optimization.metadata;
      
      return response;
      
    } catch (error) {
      this.metrics.failedRequests++;
      throw error;
    }
  }
  
  /**
   * Stream a request (with optimization)
   */
  async *stream(request: LLMRequest): AsyncIterableIterator<LLMResponse> {
    const startTime = Date.now();
    
    try {
      this.metrics.totalRequests++;
      
      // === OPTIMIZATION: Check cache first ===
      const optimization = this.optimizer.optimizeRequest(request);
      
      if (optimization.cachedResponse) {
        // For streaming, we can't really use the cache
        // (cache is for complete responses, not streams)
        // But we note it for metrics
        this.metrics.optimizationStats!.cacheHits!++;
      }
      
      // === ORIGINAL STREAMING LOGIC ===
      const optimizedRequest = optimization.request;
      const profile = optimizedRequest._profile || this.config.profile;
      const tier = optimizedRequest._tier || this.scorer.score(optimizedRequest).tier;
      
      this.metrics.profileDistribution[profile]++;
      this.metrics.tierDistribution[tier]++;
      
      const selection = this.selectModel({
        tier,
        profile,
        providers: optimizedRequest._provider ? [optimizedRequest._provider as any] : undefined
      });
      
      if (!selection) {
        throw new Error(`No suitable model found for tier: ${tier}`);
      }
      
      const provider = this.providers.getProvider(selection.provider);
      if (!provider) {
        throw new Error(`Provider not found: ${selection.provider}`);
      }
      
      // Stream with optimization
      for await (const chunk of provider.stream({
        ...optimizedRequest,
        model: selection.model
      })) {
        // Track cost
        this.metrics.totalCostUSD += chunk._costUSD || 0;
        
        // Add optimization metadata
        chunk._optimizationMetadata = optimization.metadata;
        
        yield chunk;
      }
      
      this.metrics.successfulRequests++;
      const latency = Date.now() - startTime;
      this.metrics.averageLatency = (
        (this.metrics.averageLatency * (this.metrics.successfulRequests - 1) + latency) /
        this.metrics.successfulRequests
      );
      
    } catch (error) {
      this.metrics.failedRequests++;
      throw error;
    }
  }
  
  /**
   * Select best model based on tier and profile
   */
  selectModel(criteria: SelectionCriteria): SelectedModel | null {
    const models = this.providers.getModelsForTier(criteria.tier);
    
    if (!models.length) {
      return null;
    }
    
    // Filter by profile cost constraints
    let filtered = models;
    const maxCost = this.getMaxCostForProfile(criteria.profile);
    
    filtered = filtered.filter(m => 
      (m.costInput + m.costOutput) / 2 <= maxCost
    );
    
    if (!filtered.length) {
      // Fallback to cheapest available
      filtered = models;
    }
    
    // Filter by provider if specified
    if (criteria.providers) {
      filtered = filtered.filter(m => 
        criteria.providers!.includes(m.provider as any)
      );
    }
    
    if (!filtered.length) {
      return null;
    }
    
    // Sort by cost (cheapest first)
    filtered.sort((a, b) => 
      (a.costInput + a.costOutput) - (b.costInput + b.costOutput)
    );
    
    const selected = filtered[0];
    
    return {
      model: selected.id,
      provider: selected.provider,
      cost: (selected.costInput + selected.costOutput) / 2,
      tier: selected.tier
    };
  }
  
  /**
   * Get maximum cost per 1M tokens for profile
   */
  private getMaxCostForProfile(profile: Profile): number {
    switch (profile) {
      case Profile.ECO:
        return 3; // ~$3 per 1M tokens max
      case Profile.AUTO:
        return 10; // ~$10 per 1M tokens max
      case Profile.PREMIUM:
        return 100; // ~$100 per 1M tokens max
      case Profile.FREE:
        return 0.1; // Free models only
      default:
        return 10;
    }
  }
  
  /**
   * Get current metrics
   */
  getMetrics(): RouterMetrics {
    return {
      ...this.metrics,
      cacheHitRate: this.metrics.optimizationStats!.cacheHits! / 
        Math.max(1, this.metrics.totalRequests)
    };
  }
  
  /**
   * Get optimization statistics
   */
  getOptimizationStats() {
    return this.optimizer.getStats();
  }
  
  /**
   * Health check
   */
  async getHealth(): Promise<HealthStatus> {
    const providers = this.providers.getAllProviders();
    const health: HealthStatus = {
      healthy: true,
      providers: {}
    };
    
    for (const provider of providers) {
      health.providers[provider.getType()] = await provider.healthCheck();
    }
    
    health.healthy = Object.values(health.providers).some(h => h);
    
    return health;
  }
  
  /**
   * Clear caches
   */
  clearCache(): void {
    this.optimizer.clear();
  }
}

export { UnifiedRouter };
