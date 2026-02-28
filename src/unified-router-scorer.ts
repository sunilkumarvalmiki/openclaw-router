/**
 * Unified Router - 15-Dimensional Scoring Algorithm
 * Day 1: Core classification engine (ClawRouter algorithm)
 * 
 * This scorer analyzes incoming requests across 15 dimensions and classifies
 * them into tiers: SIMPLE, MEDIUM, COMPLEX, REASONING
 * 
 * Scoring dimensions:
 * 1. Input tokens         - Request size
 * 2. Context length       - Required window
 * 3. Code percentage      - Code vs prose ratio
 * 4. Language count       - # languages involved
 * 5. Step count           - Multi-step reasoning
 * 6. Tool calls           - Functions needed
 * 7. Output requirement   - Structured output needed
 * 8. Accuracy need        - Accuracy importance (0-1)
 * 9. Latency need         - Latency importance (0-1)
 * 10. Consistency         - Critical consistency
 * 11. Reasoning model     - o1/o3 capability needed
 * 12. Vision support      - Image analysis needed
 * 13. Tool calling        - Advanced tool support
 * 14. User tier           - User subscription level
 * 15. Budget remaining    - Available budget
 */

import { 
  LLMRequest, 
  ScoringDimensions, 
  ScoringResult, 
  Tier 
} from "./unified-router-types";

export class ScoringEngine {
  /**
   * Score a request and classify it into a tier
   */
  score(request: LLMRequest): ScoringResult {
    const dimensions = this.extractDimensions(request);
    const tier = this.classifyTier(dimensions);
    const confidence = this.calculateConfidence(dimensions, tier);
    const reasoning = this.generateReasoning(dimensions, tier);
    
    return {
      tier,
      confidence,
      dimensions,
      reasoning
    };
  }
  
  /**
   * Extract all 15 dimensions from the request
   */
  private extractDimensions(request: LLMRequest): ScoringDimensions {
    const messages = request.messages || [];
    const content = this.consolidateContent(messages);
    
    return {
      // Complexity indicators
      inputTokens: this.estimateTokens(content),
      contextLength: this.estimateRequiredContext(request),
      codePercentage: this.detectCodePercentage(content),
      languageCount: this.countLanguages(content),
      
      // Reasoning indicators
      stepCount: this.detectMultiStep(content),
      toolCalls: (request.tools?.length || 0),
      requiresOutput: this.detectStructuredOutput(content),
      
      // Quality indicators
      accuracy: this.detectAccuracyRequirement(content),
      latency: this.detectLatencyRequirement(content),
      consistency: this.detectConsistency(content),
      
      // Capability indicators
      requiresReasoningModel: this.detectReasoningNeeded(content),
      requiresVision: this.detectVisionNeeded(messages),
      requiresTools: (request.tools?.length || 0) > 0,
      
      // User indicators
      userTier: "pro",                       // TODO: Get from auth context
      budgetRemaining: 100,                 // TODO: Get from wallet/account
    };
  }
  
  /**
   * Consolidate all message content into one string
   */
  private consolidateContent(messages: any[]): string {
    return messages
      .map(msg => {
        if (typeof msg.content === "string") return msg.content;
        if (Array.isArray(msg.content)) {
          return msg.content
            .filter((c: any) => c.type === "text")
            .map((c: any) => c.text)
            .join("\n");
        }
        return "";
      })
      .join("\n");
  }
  
  /**
   * Dimension 1: Estimate input tokens
   * Simple heuristic: 1 token ≈ 4 characters
   */
  private estimateTokens(content: string): number {
    return Math.ceil(content.length / 4);
  }
  
  /**
   * Dimension 2: Estimate required context window
   * Based on mentions of files, documents, large contexts
   */
  private estimateRequiredContext(request: LLMRequest): number {
    const content = this.consolidateContent(request.messages || []);
    
    if (content.includes("codebase") || content.includes("repository")) {
      return 100000; // Large codebase analysis
    }
    if (content.includes("document") || content.includes("paper")) {
      return 32000;  // Multi-document analysis
    }
    if (content.length > 20000) {
      return 32000;  // Large single document
    }
    return 8000;     // Default
  }
  
  /**
   * Dimension 3: Detect code percentage (0-1)
   * Count lines that look like code
   */
  private detectCodePercentage(content: string): number {
    const lines = content.split("\n");
    const codeIndicators = [
      /^[a-zA-Z_][a-zA-Z0-9_]*\s*=/,   // Assignment
      /^\s*function\s+/,                // Function definition
      /^\s*const\s+/,                   // Const declaration
      /^\s*class\s+/,                   // Class definition
      /^\s*if\s*\(/,                    // If statement
      /^\s*for\s*\(/,                   // For loop
      /```/,                            // Code block
    ];
    
    const codeLineCount = lines.filter(line => {
      return codeIndicators.some(regex => regex.test(line));
    }).length;
    
    return Math.min(codeLineCount / Math.max(lines.length, 1), 1.0);
  }
  
  /**
   * Dimension 4: Count programming languages mentioned
   */
  private countLanguages(content: string): number {
    const languages = [
      "python", "javascript", "typescript", "java", "c++", "rust",
      "go", "sql", "html", "css", "json", "xml", "yaml", "kotlin",
      "swift", "csharp", "php", "ruby", "perl", "bash", "shell"
    ];
    
    const mentioned = new Set(
      languages.filter(lang => 
        new RegExp(`\\b${lang}\\b`, "i").test(content)
      )
    );
    
    return mentioned.size;
  }
  
  /**
   * Dimension 5: Detect multi-step reasoning needed
   * Look for keywords indicating complex reasoning
   */
  private detectMultiStep(content: string): number {
    const phrases = [
      "step by step",
      "first then",
      "analyze",
      "compare",
      "evaluate",
      "design",
      "architecture",
      "refactor",
      "optimize",
      "debug"
    ];
    
    const count = phrases.filter(phrase => 
      new RegExp(`\\b${phrase}\\b`, "i").test(content)
    ).length;
    
    return count;
  }
  
  /**
   * Dimension 7: Detect if structured output is required
   */
  private detectStructuredOutput(content: string): boolean {
    return /\b(json|xml|csv|table|list|dict|object|schema)\b/i.test(content);
  }
  
  /**
   * Dimension 8: Detect accuracy requirement (0-1)
   * Look for keywords indicating high-accuracy needs
   */
  private detectAccuracyRequirement(content: string): number {
    const highAccuracyKeywords = [
      "critical",
      "mission-critical",
      "production",
      "security",
      "financial",
      "medical",
      "legal",
      "exact",
      "precise",
      "accuracy"
    ];
    
    const count = highAccuracyKeywords.filter(kw => 
      new RegExp(`\\b${kw}\\b`, "i").test(content)
    ).length;
    
    return Math.min(count / 2, 1.0);
  }
  
  /**
   * Dimension 9: Detect latency requirement (0-1)
   */
  private detectLatencyRequirement(content: string): number {
    const lowLatencyKeywords = [
      "fast",
      "quick",
      "real-time",
      "instant",
      "immediate",
      "responsive",
      "low latency"
    ];
    
    const count = lowLatencyKeywords.filter(kw => 
      new RegExp(`\\b${kw}\\b`, "i").test(content)
    ).length;
    
    return Math.min(count / 2, 1.0);
  }
  
  /**
   * Dimension 10: Detect consistency requirement
   */
  private detectConsistency(content: string): boolean {
    return /\b(consistent|idempotent|deterministic|repeatable)\b/i.test(content);
  }
  
  /**
   * Dimension 11: Detect if reasoning model needed
   */
  private detectReasoningNeeded(content: string): boolean {
    return /\b(reason|proof|theorem|logic|math|equation|solve)\b/i.test(content);
  }
  
  /**
   * Dimension 12: Detect if vision is needed
   */
  private detectVisionNeeded(messages: any[]): boolean {
    return messages.some(msg => {
      if (Array.isArray(msg.content)) {
        return msg.content.some((c: any) => c.type === "image_url");
      }
      return false;
    });
  }
  
  /**
   * Classify request into tier based on dimensions
   */
  private classifyTier(dims: ScoringDimensions): Tier {
    // REASONING: If reasoning capability is needed or high accuracy + complex logic
    if (dims.requiresReasoningModel || 
        (dims.accuracy > 0.8 && dims.stepCount > 2)) {
      return Tier.REASONING;
    }
    
    // COMPLEX: Long context, multi-step, high accuracy, vision, many tools
    if (dims.contextLength > 50000 ||
        dims.stepCount > 3 ||
        dims.accuracy > 0.7 ||
        dims.requiresVision ||
        (dims.toolCalls > 2)) {
      return Tier.COMPLEX;
    }
    
    // MEDIUM: Moderate complexity
    if (dims.inputTokens > 5000 ||
        dims.stepCount > 1 ||
        dims.accuracy > 0.5 ||
        dims.toolCalls > 0 ||
        dims.codePercentage > 0.3 ||
        dims.languageCount > 2) {
      return Tier.MEDIUM;
    }
    
    // SIMPLE: Everything else
    return Tier.SIMPLE;
  }
  
  /**
   * Calculate confidence in tier classification (0-1)
   */
  private calculateConfidence(dims: ScoringDimensions, tier: Tier): number {
    // TODO: Implement sophisticated confidence scoring
    // For now, return high confidence for clear cases
    
    if (tier === Tier.SIMPLE && dims.inputTokens < 500) return 0.95;
    if (tier === Tier.COMPLEX && dims.contextLength > 50000) return 0.9;
    
    return 0.75; // Default
  }
  
  /**
   * Generate human-readable reasoning for tier classification
   */
  private generateReasoning(dims: ScoringDimensions, tier: Tier): string {
    const reasons: string[] = [];
    
    if (dims.inputTokens > 5000) reasons.push("long input");
    if (dims.contextLength > 50000) reasons.push("large context needed");
    if (dims.stepCount > 1) reasons.push("multi-step reasoning");
    if (dims.accuracy > 0.7) reasons.push("high accuracy needed");
    if (dims.requiresVision) reasons.push("vision capability");
    if (dims.requiresReasoningModel) reasons.push("complex reasoning");
    
    return `Classified as ${tier}: ${reasons.join(", ") || "simple query"}`;
  }
}

export const scoringEngine = new ScoringEngine();
