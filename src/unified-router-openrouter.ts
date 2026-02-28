/**
 * Unified Router - OpenRouter Provider Implementation
 * Day 2: Multi-provider aggregator (100+ models)
 * 
 * OpenRouter provides access to 100+ models from:
 * - OpenAI (GPT-4o, o1, o3)
 * - Anthropic (Claude 3.5 Sonnet, Opus)
 * - Google (Gemini 3.1 Pro, Flash)
 * - Open-source (Llama, Mistral, etc.)
 * - xAI (Grok)
 * - DeepSeek
 * - And more...
 */

import axios, { AxiosInstance } from "axios";
import {
  AbstractProviderClient,
  ModelInfo,
  Tier,
  LLMRequest,
  LLMResponse,
  ProviderConfig
} from "./unified-router-types";

/**
 * OpenRouter Provider - Access 100+ models via single API
 */
export class OpenRouterProvider extends AbstractProviderClient {
  private client: AxiosInstance;
  private apiKey: string;
  private baseUrl: string = "https://openrouter.ai/api/v1";
  private appName: string = "unified-router";
  private appVersion: string = "0.1.0";
  
  constructor(config: ProviderConfig) {
    super("openrouter", config);
    
    // Get API key from config or environment
    this.apiKey = config.apiKey || process.env.OPENROUTER_API_KEY || "";
    this.baseUrl = config.baseUrl || "https://openrouter.ai/api/v1";
    
    if (!this.apiKey) {
      console.warn("⚠️ OpenRouter API key not set. Set OPENROUTER_API_KEY environment variable.");
      this.isHealthy = false;
    }
    
    // Create axios client with headers
    this.client = axios.create({
      baseURL: this.baseUrl,
      headers: {
        "Authorization": `Bearer ${this.apiKey}`,
        "HTTP-Referer": "https://unified-router.example.com", // Required by OpenRouter
        "X-Title": "Unified Router",
        "Content-Type": "application/json"
      },
      timeout: 120000 // 2 minute timeout
    });
  }
  
  /**
   * Initialize - Fetch available models from OpenRouter
   */
  async initialize(): Promise<void> {
    try {
      // Fetch model list
      const response = await this.client.get("/models");
      const { data: models } = response.data || { data: [] };
      
      if (!Array.isArray(models)) {
        throw new Error("Invalid model list format");
      }
      
      // Register models
      models.forEach((model: any) => {
        try {
          const modelInfo = this.mapOpenRouterModelToModelInfo(model);
          this.models.set(model.id, modelInfo);
        } catch (e) {
          // Skip models we can't parse
        }
      });
      
      this.isHealthy = true;
      console.log(`✅ OpenRouter: Loaded ${this.models.size} models`);
      
    } catch (error: any) {
      console.error(`❌ OpenRouter initialization failed: ${error.message}`);
      
      if (error.response?.status === 401) {
        console.error("   Invalid API key. Set OPENROUTER_API_KEY environment variable.");
      }
      
      this.isHealthy = false;
    }
  }
  
  /**
   * Complete a request
   */
  async complete(request: LLMRequest): Promise<LLMResponse> {
    const model = request.model || "openai/gpt-4o";
    
    try {
      // Convert to OpenRouter format (standard OpenAI format)
      const openrouterRequest = this.normalizeRequest(request, model);
      
      // Call OpenRouter API
      const response = await this.client.post("/chat/completions", openrouterRequest);
      
      // Normalize response
      return this.normalizeResponse(response, model, response.headers);
      
    } catch (error: any) {
      console.error(`OpenRouter request error: ${error.message}`);
      throw new Error(`OpenRouter request failed: ${error.message}`);
    }
  }
  
  /**
   * Stream a request
   */
  async *stream(request: LLMRequest): AsyncIterableIterator<LLMResponse> {
    const model = request.model || "openai/gpt-4o";
    
    const openrouterRequest = {
      ...this.normalizeRequest(request, model),
      stream: true
    };
    
    try {
      const response = await this.client.post("/chat/completions", openrouterRequest, {
        responseType: "stream"
      });
      
      let fullText = "";
      let totalInputTokens = 0;
      let totalOutputTokens = 0;
      
      // Process SSE stream
      for await (const chunk of response.data) {
        const lines = chunk.toString("utf8").split("\n");
        
        for (const line of lines) {
          if (!line.startsWith("data: ")) continue;
          
          const data = line.slice(6); // Remove "data: " prefix
          if (data === "[DONE]") {
            continue;
          }
          
          try {
            const parsed = JSON.parse(data);
            
            if (parsed.choices?.[0]?.delta?.content) {
              const content = parsed.choices[0].delta.content;
              fullText += content;
              
              // Estimate output tokens
              totalOutputTokens = Math.ceil(fullText.length / 4);
              
              yield {
                id: `openrouter-stream-${Date.now()}`,
                object: "text_completion.chunk",
                created: Math.floor(Date.now() / 1000),
                model,
                choices: [{
                  index: 0,
                  message: {
                    role: "assistant",
                    content
                  },
                  finish_reason: parsed.choices[0].finish_reason || null
                }],
                usage: {
                  prompt_tokens: totalInputTokens,
                  completion_tokens: totalOutputTokens,
                  total_tokens: totalInputTokens + totalOutputTokens
                },
                _selectedProvider: "openrouter",
                _costUSD: this.estimateCost(totalInputTokens, totalOutputTokens, model),
                _cached: false
              };
            }
          } catch (e) {
            // Skip invalid JSON
          }
        }
      }
    } catch (error: any) {
      throw new Error(`OpenRouter streaming failed: ${error.message}`);
    }
  }
  
  /**
   * Get available models
   */
  getModels(): ModelInfo[] {
    return Array.from(this.models.values());
  }
  
  /**
   * Health check
   */
  async healthCheck(): Promise<boolean> {
    try {
      await this.client.get("/models");
      this.isHealthy = true;
      return true;
    } catch (error) {
      this.isHealthy = false;
      return false;
    }
  }
  
  /**
   * Normalize request to OpenRouter format
   */
  private normalizeRequest(request: LLMRequest, model: string): any {
    return {
      model,
      messages: request.messages,
      temperature: request.temperature ?? 0.7,
      max_tokens: request.max_tokens ?? 2000,
      top_p: request.top_p ?? 1.0,
      frequency_penalty: request.frequency_penalty ?? 0,
      presence_penalty: request.presence_penalty ?? 0,
      stream: false,
      ...(request.tools && { tools: request.tools }),
      ...(request.tool_choice && { tool_choice: request.tool_choice })
    };
  }
  
  /**
   * Normalize response from OpenRouter
   */
  protected normalizeResponse(response: any, model: string, headers: any): LLMResponse {
    const { data } = response;
    
    // Extract pricing info from headers (OpenRouter provides cost in headers)
    const credits = parseFloat(headers["x-credits-remaining"] || "0");
    
    return {
      id: data.id || `openrouter-${Date.now()}`,
      object: data.object || "text_completion",
      created: data.created || Math.floor(Date.now() / 1000),
      model,
      choices: data.choices || [],
      usage: data.usage || {
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0
      },
      _selectedProvider: "openrouter",
      _costUSD: this.estimateCost(
        data.usage?.prompt_tokens || 0,
        data.usage?.completion_tokens || 0,
        model
      ),
      _cached: false
    };
  }
  
  /**
   * Map OpenRouter model to ModelInfo
   */
  private mapOpenRouterModelToModelInfo(model: any): ModelInfo {
    const id = model.id || "";
    
    // Infer tier from model name
    let tier = Tier.MEDIUM;
    if (id.includes("mini") || id.includes("3.5") || id.includes("flash")) {
      tier = Tier.SIMPLE;
    } else if (id.includes("4o") || id.includes("sonnet") || id.includes("opus")) {
      tier = Tier.COMPLEX;
    } else if (id.includes("o1") || id.includes("o3")) {
      tier = Tier.REASONING;
    }
    
    // Extract pricing from model
    const pricing = model.pricing || {};
    const costInput = parseFloat(pricing.prompt || "0.001");
    const costOutput = parseFloat(pricing.completion || "0.005");
    
    return {
      id,
      provider: "openrouter",
      name: model.name || id,
      tier,
      costInput: costInput * 1000, // Convert to per-1M tokens
      costOutput: costOutput * 1000,
      maxTokens: model.context_length || 128000,
      supports: {
        vision: model.architecture?.modality?.includes("image") || false,
        tools: !model.architecture?.modality?.includes("text-only") || false,
        streaming: true,
        reasoning: id.includes("o1") || id.includes("o3") || id.includes("reasoning")
      },
      avgLatency: 500,
      reliability: 0.99,
      available: true,
      lastHealthCheck: Date.now()
    };
  }
  
  /**
   * Estimate cost for a request
   */
  private estimateCost(inputTokens: number, outputTokens: number, model: string): number {
    const modelInfo = this.models.get(model);
    if (!modelInfo) {
      return 0.01; // Default estimate
    }
    
    const inputCost = (inputTokens / 1000000) * modelInfo.costInput;
    const outputCost = (outputTokens / 1000000) * modelInfo.costOutput;
    
    return inputCost + outputCost;
  }
}

/**
 * Helper: Create OpenRouter provider
 */
export const createOpenRouterProvider = (config?: Partial<ProviderConfig>): OpenRouterProvider => {
  return new OpenRouterProvider({
    type: "openrouter",
    enabled: true,
    apiKey: config?.apiKey || process.env.OPENROUTER_API_KEY,
    ...config
  });
};

export default OpenRouterProvider;
