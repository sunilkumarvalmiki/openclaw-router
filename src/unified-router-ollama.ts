/**
 * Unified Router - Ollama Provider Implementation
 * Day 2: Local LLM provider (FREE models)
 * 
 * Ollama provides access to local LLM models like:
 * - Qwen 2.5 Coder (code generation)
 * - Llama 3.1 (general purpose)
 * - Mistral (general purpose)
 * - Neural Chat (instruction following)
 */

import axios, { AxiosInstance } from "axios";
import {
  AbstractProviderClient,
  ModelInfo,
  Tier,
  LLMRequest,
  LLMResponse,
  ProviderConfig,
  Message,
  Choice
} from "./unified-router-types";

/**
 * Ollama Provider - Connect to local Ollama server
 * Default: http://localhost:11434
 */
export class OllamaProvider extends AbstractProviderClient {
  private client: AxiosInstance;
  private endpoint: string;
  private defaultModel: string = "qwen2.5-coder:latest";
  
  constructor(config: ProviderConfig) {
    super("ollama", config);
    
    // Get endpoint from config or environment
    this.endpoint = config.endpoint || process.env.OLLAMA_ENDPOINT || "http://localhost:11434";
    this.defaultModel = process.env.OLLAMA_MODEL || this.defaultModel;
    
    // Create axios client
    this.client = axios.create({
      baseURL: this.endpoint,
      timeout: 300000, // 5 minute timeout for long generations
    });
  }
  
  /**
   * Initialize - Load available models and test connection
   */
  async initialize(): Promise<void> {
    try {
      // Get list of available models
      const response = await this.client.get("/api/tags");
      const { models } = response.data;
      
      if (!Array.isArray(models) || models.length === 0) {
        console.warn("Ollama: No models found. Pull a model with: ollama pull qwen2.5-coder");
        this.isHealthy = false;
        return;
      }
      
      // Register each model
      models.forEach((model: any) => {
        const modelInfo = this.mapOllamaModelToModelInfo(model);
        this.models.set(model.name, modelInfo);
      });
      
      this.isHealthy = true;
      console.log(`✅ Ollama: Loaded ${models.length} models`);
      
    } catch (error: any) {
      console.error(`❌ Ollama initialization failed: ${error.message}`);
      this.isHealthy = false;
      
      // Try to provide helpful error message
      if (error.code === "ECONNREFUSED") {
        console.error("   Start Ollama with: ollama serve");
      }
    }
  }
  
  /**
   * Complete a request synchronously
   */
  async complete(request: LLMRequest): Promise<LLMResponse> {
    const model = request.model || this.defaultModel;
    
    // Convert LLMRequest to Ollama format
    const ollamaRequest = this.normalizeRequest(request, model);
    
    try {
      // Call Ollama API
      const response = await this.client.post("/api/chat", ollamaRequest, {
        timeout: 300000
      });
      
      // Normalize response
      return this.normalizeResponse(response.data, model);
      
    } catch (error: any) {
      throw new Error(`Ollama request failed: ${error.message}`);
    }
  }
  
  /**
   * Stream a request
   */
  async *stream(request: LLMRequest): AsyncIterableIterator<LLMResponse> {
    const model = request.model || this.defaultModel;
    const ollamaRequest = {
      ...this.normalizeRequest(request, model),
      stream: true
    };
    
    try {
      const response = await this.client.post("/api/chat", ollamaRequest, {
        responseType: "stream",
        timeout: 300000
      });
      
      let fullText = "";
      let tokenCount = 0;
      
      // Process stream
      for await (const chunk of response.data) {
        const lines = chunk.toString("utf8").split("\n");
        
        for (const line of lines) {
          if (!line.trim()) continue;
          
          try {
            const data = JSON.parse(line);
            if (data.message?.content) {
              fullText += data.message.content;
              
              // Estimate tokens (roughly)
              tokenCount = Math.ceil(fullText.length / 4);
              
              // Yield chunk response
              yield {
                id: `ollama-stream-${Date.now()}`,
                object: "text_completion.chunk",
                created: Math.floor(Date.now() / 1000),
                model,
                choices: [{
                  index: 0,
                  message: {
                    role: "assistant",
                    content: data.message.content
                  },
                  finish_reason: data.done ? "stop" : null
                }],
                usage: {
                  prompt_tokens: 0,
                  completion_tokens: tokenCount,
                  total_tokens: tokenCount
                },
                _selectedProvider: "ollama",
                _costUSD: 0,
                _cached: false
              };
              
              if (data.done) {
                return;
              }
            }
          } catch (e) {
            // Skip invalid JSON lines
          }
        }
      }
    } catch (error: any) {
      throw new Error(`Ollama streaming failed: ${error.message}`);
    }
  }
  
  /**
   * Get available models
   */
  getModels(): ModelInfo[] {
    return Array.from(this.models.values());
  }
  
  /**
   * Health check - verify Ollama is running
   */
  async healthCheck(): Promise<boolean> {
    try {
      await this.client.get("/api/tags");
      this.isHealthy = true;
      return true;
    } catch (error) {
      this.isHealthy = false;
      return false;
    }
  }
  
  /**
   * Normalize Ollama request to standard format
   */
  private normalizeRequest(request: LLMRequest, model: string): any {
    return {
      model,
      messages: request.messages.map(msg => ({
        role: msg.role,
        content: typeof msg.content === "string" ? msg.content : ""
      })),
      temperature: request.temperature ?? 0.7,
      top_p: request.top_p ?? 1.0,
      stream: false
    };
  }
  
  /**
   * Normalize Ollama response to standard LLMResponse
   */
  protected normalizeResponse(raw: any, model: string): LLMResponse {
    const message = raw.message?.content || "";
    const promptTokens = Math.ceil((raw.prompt_eval_count || 0) / 1) || 0;
    const completionTokens = Math.ceil((raw.eval_count || 0) / 1) || 0;
    
    return {
      id: `ollama-${Date.now()}-${Math.random().toString(36).slice(2)}`,
      object: "text_completion",
      created: Math.floor(Date.now() / 1000),
      model,
      choices: [{
        index: 0,
        message: {
          role: "assistant",
          content: message
        },
        finish_reason: raw.done ? "stop" : "length"
      }],
      usage: {
        prompt_tokens: promptTokens,
        completion_tokens: completionTokens,
        total_tokens: promptTokens + completionTokens
      },
      _selectedProvider: "ollama",
      _costUSD: 0, // Ollama is free (local)
      _cached: false
    };
  }
  
  /**
   * Map Ollama model to ModelInfo
   */
  private mapOllamaModelToModelInfo(ollamaModel: any): ModelInfo {
    const name = ollamaModel.name || "";
    
    // Infer tier based on model name/size
    let tier = Tier.MEDIUM;
    if (name.includes("7b") || name.includes("8b")) {
      tier = Tier.SIMPLE;
    } else if (name.includes("13b") || name.includes("14b")) {
      tier = Tier.MEDIUM;
    } else if (name.includes("70b")) {
      tier = Tier.COMPLEX;
    }
    
    return {
      id: name,
      provider: "ollama",
      name: name.split(":")[0], // Remove tag
      tier,
      costInput: 0,  // Free - local
      costOutput: 0, // Free - local
      maxTokens: 32000, // Typical context
      supports: {
        vision: name.includes("vision"),
        tools: false, // Ollama doesn't support function calling yet
        streaming: true,
        reasoning: name.includes("reasoning") || name.includes("deepseek")
      },
      avgLatency: 500, // Estimate: 500-2000ms depending on model
      reliability: 0.95,
      available: true,
      lastHealthCheck: Date.now()
    };
  }
}

/**
 * Helper: Create Ollama provider from config
 */
export const createOllamaProvider = (config?: Partial<ProviderConfig>): OllamaProvider => {
  return new OllamaProvider({
    type: "ollama",
    enabled: true,
    endpoint: config?.endpoint || process.env.OLLAMA_ENDPOINT,
    ...config
  });
};

export default OllamaProvider;
