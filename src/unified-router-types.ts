/**
 * Unified Router - Core TypeScript Types & Interfaces
 * Day 1: Foundation types for scoring, providers, and routing
 */

// ============================================================================
// TIER & SCORING TYPES (ClawRouter 15-dimensional scoring)
// ============================================================================

export enum Tier {
  SIMPLE = "simple",      // Simple queries, math, logic
  MEDIUM = "medium",      // Multi-step reasoning, code review
  COMPLEX = "complex",    // Long-form analysis, architecture
  REASONING = "reasoning" // OpenAI o1/o3 style reasoning
}

export enum Profile {
  ECO = "eco",            // Cheapest possible (95-100% savings)
  AUTO = "auto",          // Balanced (74-100% savings) - DEFAULT
  PREMIUM = "premium",    // Best quality (0% savings)
  FREE = "free"           // Free tier only
}

/** 15-dimensional scoring dimensions (ClawRouter algorithm) */
export interface ScoringDimensions {
  // Complexity indicators
  inputTokens: number;            // Input length
  contextLength: number;          // Required context window
  codePercentage: number;         // % code vs prose
  languageCount: number;          // # languages in request
  
  // Reasoning indicators
  stepCount: number;              // Multi-step reasoning needed
  toolCalls: number;              // Tool/function calls needed
  requiresOutput: boolean;        // Structured output needed
  
  // Quality indicators
  accuracy: number;               // Accuracy requirement (0-1)
  latency: number;                // Latency requirement (0-1)
  consistency: boolean;           // Consistency critical?
  
  // Capability indicators
  requiresReasoningModel: boolean; // Needs o1/o3/reasoning
  requiresVision: boolean;        // Needs image support
  requiresTools: boolean;         // Needs tool calling
  
  // User indicators
  userTier: string;               // free/pro/enterprise
  budgetRemaining: number;        // $ remaining this period
  modelPreference?: string;       // User's preferred model
}

export interface ScoringResult {
  tier: Tier;
  confidence: number;            // 0-1 confidence in tier classification
  dimensions: ScoringDimensions;
  reasoning: string;             // Explanation of tier choice
}

// ============================================================================
// REQUEST & RESPONSE TYPES
// ============================================================================

export interface LLMRequest {
  model?: string;                 // Optional model override
  messages: Message[];
  temperature?: number;
  max_tokens?: number;
  tools?: Tool[];
  tool_choice?: string | "auto" | "required";
  top_p?: number;
  frequency_penalty?: number;
  presence_penalty?: number;
  stop?: string[];
  stream?: boolean;
  user?: string;
  
  // Unified router metadata
  _profile?: Profile;             // Routing profile (eco/auto/premium/free)
  _tier?: Tier;                   // Pre-computed tier (skip scoring)
  _provider?: string;             // Provider override
}

export interface Message {
  role: "user" | "assistant" | "system";
  content: string | ContentBlock[];
}

export interface ContentBlock {
  type: "text" | "image_url" | "tool_result";
  text?: string;
  image_url?: { url: string; detail?: string };
  tool_use_id?: string;
  content?: string;
}

export interface Tool {
  type: "function";
  function: {
    name: string;
    description: string;
    parameters: object;
  };
}

export interface LLMResponse {
  id: string;
  object: string;
  created: number;
  model: string;
  choices: Choice[];
  usage: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
  
  // Router metadata
  _selectedModel?: string;
  _selectedProvider?: string;
  _costUSD?: number;
  _tier?: Tier;
  _cached?: boolean;
  _profile?: Profile;
}

export interface Choice {
  index: number;
  message: Message;
  finish_reason: string;
}

// ============================================================================
// PROVIDER TYPES
// ============================================================================

export type ProviderType =
  | "databricks"
  | "bedrock"
  | "openrouter"
  | "ollama"
  | "llamacpp"
  | "lmstudio"
  | "azure-openai"
  | "azure-anthropic";

export interface ProviderConfig {
  type: ProviderType;
  enabled: boolean;
  apiKey?: string;
  baseUrl?: string;
  endpoint?: string;
  
  // Provider-specific
  region?: string;                // AWS region for Bedrock
  deploymentId?: string;          // Azure deployment ID
  resourceName?: string;          // Azure resource name
  
  // Rate limiting
  rateLimit?: number;             // Requests per minute
  maxConcurrent?: number;         // Concurrent requests
  
  // Health check
  healthCheckInterval?: number;   // ms
  timeout?: number;               // ms
}

export interface ModelInfo {
  id: string;
  provider: ProviderType;
  name: string;
  tier: Tier;
  
  // Pricing (per 1M tokens)
  costInput: number;              // $ per 1M input tokens
  costOutput: number;             // $ per 1M output tokens
  
  // Capabilities
  maxTokens: number;              // Context window
  supports: {
    vision: boolean;
    tools: boolean;
    streaming: boolean;
    reasoning: boolean;            // o1/o3 style
  };
  
  // Performance
  avgLatency: number;             // ms (estimated)
  reliability: number;            // 0-1 uptime %
  
  // Availability
  available: boolean;
  lastHealthCheck: number;        // timestamp
}

export interface ProviderMetrics {
  provider: ProviderType;
  requestCount: number;
  successCount: number;
  errorCount: number;
  avgLatency: number;
  totalCostUSD: number;
  lastUsed: number;
}

// ============================================================================
// OPTIMIZATION TYPES
// ============================================================================

export interface OptimizationOptions {
  enableCache: boolean;           // Semantic cache
  enableDedup: boolean;           // Request deduplication
  enableCompress: boolean;        // Memory compression
  filterTools: boolean;           // Smart tool filtering
  estimateTokens: boolean;        // Pre-estimate tokens
}

export interface OptimizationResult {
  originalTokens: number;
  optimizedTokens: number;
  savings: number;                // % reduction
  techniques: string[];           // Which optimizations applied
  reason: string;
}

// ============================================================================
// PAYMENT TYPES (x402)
// ============================================================================

export interface PaymentConfig {
  enabled: boolean;
  walletKey?: string;             // Private key (auto-generated if not set)
  rpcUrl?: string;                // Base L2 RPC
  paymentThreshold?: number;      // Min $ to trigger payment
}

export interface PaymentRequest {
  provider: ProviderType;
  model: string;
  tokenCount: number;
  estimatedCost: number;
}

export interface PaymentResult {
  txHash: string;
  confirmed: boolean;
  timestamp: number;
  gasCost: number;
}

// ============================================================================
// SELECTION TYPES
// ============================================================================

export interface SelectionCriteria {
  tier: Tier;
  profile: Profile;
  budget?: number;                // Max $ to spend
  providers?: ProviderType[];      // Preferred providers
  excludeProviders?: ProviderType[];
  forceLocal?: boolean;           // Only local providers
}

export interface SelectedModel {
  model: string;
  provider: ProviderType;
  tier: Tier;
  estimatedCost: number;
  reason: string;
}

// ============================================================================
// ERROR TYPES
// ============================================================================

export class RouterError extends Error {
  code: string;
  statusCode: number;
  
  constructor(code: string, message: string, statusCode: number = 500) {
    super(message);
    this.code = code;
    this.statusCode = statusCode;
  }
}

export interface ErrorResponse {
  error: {
    code: string;
    message: string;
    statusCode: number;
  };
}

// ============================================================================
// ROUTER CONFIG
// ============================================================================

export interface RouterConfig {
  // Core
  profile: Profile;
  logLevel: "debug" | "info" | "warn" | "error";
  
  // Providers
  providers: ProviderConfig[];
  defaultProvider?: ProviderType;
  
  // Optimization
  optimization: OptimizationOptions;
  
  // Payment (x402)
  payment: PaymentConfig;
  
  // Observability
  metrics: {
    enabled: boolean;
    port?: number;
  };
  
  // Performance
  maxConcurrent: number;
  requestTimeout: number;         // ms
}

export interface RouterMetrics {
  totalRequests: number;
  successfulRequests: number;
  failedRequests: number;
  totalCostUSD: number;
  averageLatency: number;
  cacheHitRate: number;
  profileDistribution: Record<Profile, number>;
  tierDistribution: Record<Tier, number>;
  providerDistribution: Record<ProviderType, number>;
}

export interface HealthStatus {
  status: "healthy" | "degraded" | "unhealthy";
  providers: Record<ProviderType, boolean>;
  metrics: RouterMetrics;
  lastCheck: number;
}
