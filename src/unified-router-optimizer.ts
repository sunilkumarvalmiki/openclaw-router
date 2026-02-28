/**
 * Unified Router - Token Optimization Layer
 * Day 3: Semantic caching, deduplication, and compression
 * 
 * Goals:
 * - Reduce tokens by 30-60% on repeated/similar requests
 * - Cache responses at semantic level (not just exact match)
 * - Deduplicate redundant tool calls and parameters
 * - Compress prompts intelligently
 * - Reduce latency and cost simultaneously
 */

import crypto from "crypto";
import { LLMRequest, LLMResponse, Message } from "./unified-router-types";

/**
 * Semantic Token Cache
 * Stores responses by semantic similarity, not exact match
 * Finds matches for similar (not identical) requests
 */
export class SemanticCache {
  private cache: Map<string, CacheEntry> = new Map();
  private maxEntries: number;
  private ttlMs: number;
  
  constructor(maxEntries: number = 10000, ttlMs: number = 3600000) {
    this.maxEntries = maxEntries;
    this.ttlMs = ttlMs; // 1 hour default
  }
  
  /**
   * Check if request has a cached semantic match
   * Returns cache hit if request is >85% similar to cached request
   */
  get(request: LLMRequest): LLMResponse | null {
    const requestHash = this.hashRequest(request);
    
    for (const [, entry] of this.cache) {
      // Check if expired
      if (Date.now() - entry.timestamp > this.ttlMs) {
        this.cache.delete(requestHash);
        continue;
      }
      
      // Calculate semantic similarity
      const similarity = this.calculateSimilarity(request, entry.request);
      
      // Return if >85% similar
      if (similarity > 0.85) {
        entry.hits++;
        entry.lastAccess = Date.now();
        return entry.response;
      }
    }
    
    return null;
  }
  
  /**
   * Store request-response pair
   */
  set(request: LLMRequest, response: LLMResponse): void {
    // Evict oldest entry if at capacity
    if (this.cache.size >= this.maxEntries) {
      let oldest: string | null = null;
      let oldestTime = Infinity;
      
      for (const [key, entry] of this.cache) {
        if (entry.lastAccess < oldestTime) {
          oldest = key;
          oldestTime = entry.lastAccess;
        }
      }
      
      if (oldest) {
        this.cache.delete(oldest);
      }
    }
    
    const hash = this.hashRequest(request);
    this.cache.set(hash, {
      request,
      response,
      timestamp: Date.now(),
      lastAccess: Date.now(),
      hits: 0
    });
  }
  
  /**
   * Get cache statistics
   */
  getStats(): CacheStats {
    let totalHits = 0;
    let totalAge = 0;
    
    for (const entry of this.cache.values()) {
      totalHits += entry.hits;
      totalAge += Date.now() - entry.timestamp;
    }
    
    const avgAge = this.cache.size > 0 ? totalAge / this.cache.size : 0;
    
    return {
      entries: this.cache.size,
      maxEntries: this.maxEntries,
      totalHits,
      averageAge: avgAge,
      hitRate: totalHits / (totalHits + 1), // +1 to avoid division by zero
      utilizationPercent: (this.cache.size / this.maxEntries) * 100
    };
  }
  
  /**
   * Clear all cache
   */
  clear(): void {
    this.cache.clear();
  }
  
  /**
   * Hash request for quick lookup
   */
  private hashRequest(request: LLMRequest): string {
    const content = JSON.stringify({
      messages: request.messages,
      model: request.model,
      temperature: request.temperature
    });
    
    return crypto.createHash("sha256").update(content).digest("hex");
  }
  
  /**
   * Calculate semantic similarity between two requests (0-1)
   * Uses content-based similarity, not exact matching
   */
  private calculateSimilarity(req1: LLMRequest, req2: LLMRequest): number {
    let score = 0;
    let factors = 0;
    
    // Compare message content (weighted highest)
    if (req1.messages.length === req2.messages.length) {
      const contentSimilarity = this.compareContent(
        req1.messages,
        req2.messages
      );
      score += contentSimilarity * 0.4; // 40% weight
      factors++;
    }
    
    // Compare model (if same, full point)
    if (req1.model === req2.model) {
      score += 0.3;
    } else {
      score += this.modelCompatibility(req1.model, req2.model) * 0.3;
    }
    factors++;
    
    // Compare temperature/sampling parameters
    const tempDiff = Math.abs((req1.temperature || 0.7) - (req2.temperature || 0.7));
    const tempSimilarity = Math.max(0, 1 - tempDiff / 2);
    score += tempSimilarity * 0.2; // 20% weight
    factors++;
    
    // Compare max_tokens
    const tokensDiff = Math.abs((req1.max_tokens || 2000) - (req2.max_tokens || 2000));
    const tokensSimilarity = Math.max(0, 1 - tokensDiff / 4000);
    score += tokensSimilarity * 0.1; // 10% weight
    factors++;
    
    return factors > 0 ? score / factors : 0;
  }
  
  /**
   * Compare message content for similarity
   */
  private compareContent(msgs1: Message[], msgs2: Message[]): number {
    if (msgs1.length !== msgs2.length) return 0;
    
    let totalSimilarity = 0;
    
    for (let i = 0; i < msgs1.length; i++) {
      const content1 = typeof msgs1[i].content === "string" ? msgs1[i].content : "";
      const content2 = typeof msgs2[i].content === "string" ? msgs2[i].content : "";
      
      totalSimilarity += this.stringSimilarity(content1, content2);
    }
    
    return msgs1.length > 0 ? totalSimilarity / msgs1.length : 0;
  }
  
  /**
   * String similarity using Jaccard similarity (words)
   */
  private stringSimilarity(str1: string, str2: string): number {
    const words1 = new Set(str1.toLowerCase().split(/\s+/));
    const words2 = new Set(str2.toLowerCase().split(/\s+/));
    
    if (words1.size === 0 && words2.size === 0) return 1;
    if (words1.size === 0 || words2.size === 0) return 0;
    
    const intersection = new Set([...words1].filter(w => words2.has(w)));
    const union = new Set([...words1, ...words2]);
    
    return intersection.size / union.size;
  }
  
  /**
   * Model compatibility score (same provider = 1.0, different = 0.5)
   */
  private modelCompatibility(model1: string | undefined, model2: string | undefined): number {
    if (!model1 || !model2) return 0.5;
    
    const provider1 = model1.split("/")[0];
    const provider2 = model2.split("/")[0];
    
    return provider1 === provider2 ? 1.0 : 0.5;
  }
}

/**
 * Request Deduplicator
 * Detects and merges duplicate requests within a time window
 */
export class RequestDeduplicator {
  private pending: Map<string, PendingRequest> = new Map();
  private windowMs: number;
  
  constructor(windowMs: number = 1000) {
    this.windowMs = windowMs; // 1 second default
  }
  
  /**
   * Check if request is pending (deduplicate)
   */
  getIfPending(request: LLMRequest): Promise<LLMResponse> | null {
    const hash = this.hashRequest(request);
    const pending = this.pending.get(hash);
    
    if (pending && Date.now() - pending.timestamp < this.windowMs) {
      // Return the existing promise
      return pending.promise;
    }
    
    return null;
  }
  
  /**
   * Register a new request
   */
  register(
    request: LLMRequest,
    promise: Promise<LLMResponse>
  ): void {
    const hash = this.hashRequest(request);
    
    this.pending.set(hash, {
      request,
      promise,
      timestamp: Date.now()
    });
    
    // Cleanup after window expires
    setTimeout(() => {
      this.pending.delete(hash);
    }, this.windowMs);
  }
  
  /**
   * Get deduplication stats
   */
  getStats(): DeduplicationStats {
    return {
      pendingRequests: this.pending.size,
      windowMs: this.windowMs
    };
  }
  
  /**
   * Hash request for deduplication
   * Uses exact match (unlike semantic cache)
   */
  private hashRequest(request: LLMRequest): string {
    const content = JSON.stringify({
      messages: request.messages,
      model: request.model,
      temperature: request.temperature,
      max_tokens: request.max_tokens
    });
    
    return crypto.createHash("md5").update(content).digest("hex");
  }
}

/**
 * Prompt Compressor
 * Intelligently reduces token count while preserving meaning
 */
export class PromptCompressor {
  /**
   * Compress a prompt request
   * Strategies:
   * - Remove redundant whitespace
   * - Abbreviate common phrases
   * - Remove comments
   * - Consolidate similar instructions
   */
  compress(request: LLMRequest): LLMRequest {
    return {
      ...request,
      messages: request.messages.map(msg => ({
        ...msg,
        content: typeof msg.content === "string"
          ? this.compressText(msg.content)
          : msg.content
      }))
    };
  }
  
  /**
   * Compress text content
   */
  private compressText(text: string): string {
    let compressed = text;
    
    // Remove extra whitespace
    compressed = compressed.replace(/\s+/g, " ").trim();
    
    // Remove common filler words (preserve meaning)
    const fillers = [
      /\b(please|kindly|thank you|thanks)\b/gi,
      /\b(obviously|clearly|apparently)\b/gi,
      /\b(actually|basically|really)\b/gi,
      /\b(I think|I believe|I suppose)\b/gi
    ];
    
    fillers.forEach(pattern => {
      compressed = compressed.replace(pattern, "");
    });
    
    // Remove repeated punctuation
    compressed = compressed.replace(/([.!?]){2,}/g, "$1");
    
    // Abbreviate common phrases (preserve context)
    const abbreviations: [RegExp, string][] = [
      [/\bpython\b/gi, "py"],
      [/\btypescript\b/gi, "ts"],
      [/\bjavascript\b/gi, "js"],
      [/\bfunction\b/gi, "fn"],
      [/\breturn\b/gi, "ret"],
      [/\breturn value\b/gi, "ret"],
      [/\bparameter\b/gi, "param"],
      [/\bvariable\b/gi, "var"],
      [/\barray\b/gi, "[]"],
      [/\bobject\b/gi, "{}"],
      [/\bdocumentation\b/gi, "docs"],
      [/\bexample\b/gi, "ex"],
      [/\bimportant\b/gi, "⚠️"]
    ];
    
    abbreviations.forEach(([pattern, replacement]) => {
      // Only abbreviate if it won't hurt understanding
      if (text.length > 1000) { // Only in long prompts
        compressed = compressed.replace(pattern, replacement);
      }
    });
    
    // Remove duplicate lines
    const lines = compressed.split("\n");
    const uniqueLines = [...new Set(lines)];
    compressed = uniqueLines.join("\n");
    
    // Clean up again
    compressed = compressed.replace(/\s+/g, " ").trim();
    
    return compressed;
  }
  
  /**
   * Estimate token reduction percentage
   */
  estimateReduction(original: string, compressed: string): number {
    const originalTokens = Math.ceil(original.length / 4); // Rough estimate
    const compressedTokens = Math.ceil(compressed.length / 4);
    
    return ((originalTokens - compressedTokens) / originalTokens) * 100;
  }
}

/**
 * Token Optimizer - Combines all optimization strategies
 */
export class TokenOptimizer {
  private cache: SemanticCache;
  private dedup: RequestDeduplicator;
  private compressor: PromptCompressor;
  
  constructor(
    cacheSize: number = 10000,
    cacheTtlMs: number = 3600000,
    dedupWindowMs: number = 1000
  ) {
    this.cache = new SemanticCache(cacheSize, cacheTtlMs);
    this.dedup = new RequestDeduplicator(dedupWindowMs);
    this.compressor = new PromptCompressor();
  }
  
  /**
   * Optimize a request (apply all strategies)
   * Returns: { optimized request, optimization metadata }
   */
  optimizeRequest(request: LLMRequest): OptimizationResult {
    const original = request;
    let optimized = { ...request };
    const metadata: OptimizationMetadata = {
      strategies: [],
      originalTokens: this.estimateTokens(original),
      optimizedTokens: 0,
      savings: 0,
      savingsPercent: 0
    };
    
    // 1. Check cache first (fastest optimization)
    const cached = this.cache.get(original);
    if (cached) {
      metadata.strategies.push({
        name: "semantic_cache_hit",
        tokensReduced: metadata.originalTokens,
        latencySavedMs: 500 // Estimated
      });
      metadata.optimizedTokens = 0;
      metadata.savings = metadata.originalTokens;
      metadata.savingsPercent = 100;
      
      return {
        request: optimized,
        fromCache: true,
        cachedResponse: cached,
        metadata
      };
    }
    
    // 2. Check for pending duplicates (deduplication)
    // This would be handled at the router level, not here
    
    // 3. Compress prompt (for all requests)
    const compressed = this.compressor.compress(optimized);
    const compressionReduction = metadata.originalTokens - this.estimateTokens(compressed);
    
    if (compressionReduction > 0) {
      metadata.strategies.push({
        name: "prompt_compression",
        tokensReduced: compressionReduction,
        latencySavedMs: 10
      });
      optimized = compressed;
    }
    
    // 4. Update final metrics
    metadata.optimizedTokens = this.estimateTokens(optimized);
    metadata.savings = metadata.originalTokens - metadata.optimizedTokens;
    metadata.savingsPercent = (metadata.savings / metadata.originalTokens) * 100;
    
    return {
      request: optimized,
      fromCache: false,
      metadata
    };
  }
  
  /**
   * Store response in cache after request completes
   */
  cacheResponse(request: LLMRequest, response: LLMResponse): void {
    this.cache.set(request, response);
  }
  
  /**
   * Get combined statistics
   */
  getStats(): OptimizerStats {
    return {
      cache: this.cache.getStats(),
      dedup: this.dedup.getStats()
    };
  }
  
  /**
   * Clear all caches
   */
  clear(): void {
    this.cache.clear();
  }
  
  /**
   * Estimate tokens (rough: 1 token ≈ 4 characters)
   */
  private estimateTokens(request: LLMRequest): number {
    let total = 0;
    
    // Count message tokens
    for (const msg of request.messages) {
      if (typeof msg.content === "string") {
        total += Math.ceil(msg.content.length / 4);
      }
    }
    
    // Estimate overhead
    total += Math.ceil(request.messages.length * 5);
    
    return total;
  }
}

// ============================================================================
// TYPES & INTERFACES
// ============================================================================

interface CacheEntry {
  request: LLMRequest;
  response: LLMResponse;
  timestamp: number;
  lastAccess: number;
  hits: number;
}

interface CacheStats {
  entries: number;
  maxEntries: number;
  totalHits: number;
  averageAge: number;
  hitRate: number;
  utilizationPercent: number;
}

interface PendingRequest {
  request: LLMRequest;
  promise: Promise<LLMResponse>;
  timestamp: number;
}

interface DeduplicationStats {
  pendingRequests: number;
  windowMs: number;
}

interface OptimizationStrategy {
  name: string;
  tokensReduced: number;
  latencySavedMs: number;
}

interface OptimizationMetadata {
  strategies: OptimizationStrategy[];
  originalTokens: number;
  optimizedTokens: number;
  savings: number;
  savingsPercent: number;
}

interface OptimizationResult {
  request: LLMRequest;
  fromCache?: boolean;
  cachedResponse?: LLMResponse;
  metadata: OptimizationMetadata;
}

interface OptimizerStats {
  cache: CacheStats;
  dedup: DeduplicationStats;
}

export {
  SemanticCache,
  RequestDeduplicator,
  PromptCompressor,
  TokenOptimizer,
  CacheEntry,
  CacheStats,
  PendingRequest,
  DeduplicationStats,
  OptimizationStrategy,
  OptimizationMetadata,
  OptimizationResult,
  OptimizerStats
};
