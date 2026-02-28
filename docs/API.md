# OpenClaw Router -- API Reference

> **Base URL**: `https://sunilkumarvalmiki.xyz` (production) or `http://localhost:8080` (development)
>
> **Version**: 0.1.0

---

## Table of Contents

- [Authentication](#authentication)
- [Endpoints](#endpoints)
  - [POST /v1/chat/completions](#post-v1chatcompletions)
  - [POST /v1/messages](#post-v1messages)
  - [GET /health](#get-health)
  - [GET /ready](#get-ready)
  - [GET /metrics](#get-metrics)
- [Router Extensions](#router-extensions)
  - [Cost Profiles](#cost-profiles)
  - [Tier Hints](#tier-hints)
  - [Router Metadata](#router-metadata)
- [Error Responses](#error-responses)
- [Rate Limiting](#rate-limiting)

---

## Authentication

All `/v1/*` endpoints require a valid API key passed via the `Authorization` header:

```
Authorization: Bearer <your-api-key>
```

Health and readiness endpoints (`/health`, `/ready`) do not require authentication.

---

## Endpoints

### POST /v1/chat/completions

OpenAI-compatible chat completion endpoint. This is the primary API endpoint. It accepts the standard OpenAI chat completion request format with optional routing extensions, scores the request using the 15-dimensional scoring engine, selects the best provider/model, and returns the response with router metadata.

**URL**: `/v1/chat/completions`

**Method**: `POST`

**Content-Type**: `application/json`

#### Request Body

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `model` | `string` | No | Model identifier (e.g., `"gpt-4o"`, `"gemini-2.5-pro"`). When omitted, the router selects the best model based on request analysis. |
| `messages` | `Message[]` | **Yes** | Array of conversation messages. |
| `temperature` | `number` | No | Sampling temperature (0.0--2.0). |
| `max_tokens` | `integer` | No | Maximum number of tokens to generate. |
| `stream` | `boolean` | No | Whether to stream the response. |
| `top_p` | `number` | No | Nucleus sampling parameter. |
| `tools` | `Tool[]` | No | Array of tool/function definitions available to the model. |
| `tool_choice` | `object` | No | Controls how the model uses tools. |
| `response_format` | `object` | No | Specifies the output format (e.g., JSON mode). |
| `x_cost_profile` | `string` | No | Router extension: cost profile hint. One of `"eco"`, `"auto"`, `"premium"`, `"free"`. Defaults to `"auto"`. See [Cost Profiles](#cost-profiles). |
| `x_tier_hint` | `string` | No | Router extension: user tier hint. One of `"free"`, `"pro"`, `"enterprise"`. See [Tier Hints](#tier-hints). |

#### Message Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `role` | `string` | **Yes** | One of `"system"`, `"user"`, `"assistant"`, `"tool"`. |
| `content` | `string \| ContentPart[]` | No | Text content or array of content parts (for multi-modal/vision). |
| `name` | `string` | No | Optional name for the message author. |
| `tool_calls` | `ToolCall[]` | No | Tool calls made by the assistant. |
| `tool_call_id` | `string` | No | ID of the tool call this message is a response to. |

#### Tool Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | `string` | **Yes** | Always `"function"`. |
| `function` | `FunctionDef` | **Yes** | Function definition. |

#### FunctionDef Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | `string` | **Yes** | Function name. |
| `description` | `string` | No | Human-readable description. |
| `parameters` | `object` | No | JSON Schema describing the function parameters. |

#### Response Body

| Field | Type | Description |
|-------|------|-------------|
| `id` | `string` | Unique identifier for the completion (e.g., `"chatcmpl-abc123"`). |
| `object` | `string` | Always `"chat.completion"`. |
| `created` | `integer` | Unix timestamp of when the completion was created. |
| `model` | `string` | The model that generated the completion. |
| `choices` | `Choice[]` | Array of completion choices. |
| `usage` | `Usage` | Token usage statistics. |
| `x_router_metadata` | `RouterMetadata` | Router-specific metadata. See [Router Metadata](#router-metadata). |

#### Choice Object

| Field | Type | Description |
|-------|------|-------------|
| `index` | `integer` | Index of this choice in the array. |
| `message` | `Message` | The generated message. |
| `finish_reason` | `string` | Why the model stopped: `"stop"`, `"length"`, `"tool_calls"`, etc. |

#### Usage Object

| Field | Type | Description |
|-------|------|-------------|
| `prompt_tokens` | `integer` | Number of tokens in the prompt. |
| `completion_tokens` | `integer` | Number of tokens in the completion. |
| `total_tokens` | `integer` | Total tokens used. |

#### Example: Simple Chat Completion

```bash
curl -X POST https://sunilkumarvalmiki.xyz/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "messages": [
      {"role": "user", "content": "What is the capital of France?"}
    ]
  }'
```

Response:

```json
{
  "id": "chatcmpl-abc123",
  "object": "chat.completion",
  "created": 1709312400,
  "model": "gpt-4o-mini",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "The capital of France is Paris."
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 12,
    "completion_tokens": 8,
    "total_tokens": 20
  },
  "x_router_metadata": {
    "provider": "openai",
    "tier": "simple",
    "cost_profile": "auto",
    "cost_usd": 0.0000042,
    "cache_hit": false,
    "latency_ms": 432,
    "scoring_ms": 0.015
  }
}
```

#### Example: With Cost Profile and Tool Calling

```bash
curl -X POST https://sunilkumarvalmiki.xyz/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "gpt-4o",
    "x_cost_profile": "eco",
    "x_tier_hint": "pro",
    "messages": [
      {"role": "system", "content": "You are a helpful assistant."},
      {"role": "user", "content": "What is the weather in Tokyo?"}
    ],
    "tools": [
      {
        "type": "function",
        "function": {
          "name": "get_weather",
          "description": "Get the current weather for a location",
          "parameters": {
            "type": "object",
            "properties": {
              "location": {"type": "string", "description": "City name"}
            },
            "required": ["location"]
          }
        }
      }
    ],
    "temperature": 0.7,
    "max_tokens": 1000
  }'
```

#### Example: Vision (Multi-Modal)

```bash
curl -X POST https://sunilkumarvalmiki.xyz/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "messages": [
      {
        "role": "user",
        "content": [
          {"type": "text", "text": "What is in this image?"},
          {"type": "image_url", "image_url": {"url": "https://example.com/photo.jpg"}}
        ]
      }
    ]
  }'
```

#### Example: Maximum Savings (ECO + Free Tier)

```bash
curl -X POST https://sunilkumarvalmiki.xyz/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "x_cost_profile": "free",
    "messages": [
      {"role": "user", "content": "Summarize this paragraph in one sentence."}
    ]
  }'
```

---

### POST /v1/messages

**Status**: Planned

Anthropic Messages API-compatible endpoint. Will accept the Anthropic message format and internally translate to the unified format before routing.

**URL**: `/v1/messages`

**Method**: `POST`

#### Request Body (Planned)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `model` | `string` | **Yes** | Model identifier. |
| `messages` | `AnthropicMessage[]` | **Yes** | Array of messages in Anthropic format. |
| `max_tokens` | `integer` | **Yes** | Maximum tokens to generate. |
| `system` | `string` | No | System prompt. |
| `temperature` | `number` | No | Sampling temperature. |
| `stream` | `boolean` | No | Whether to stream the response. |
| `tools` | `object[]` | No | Tool definitions. |

#### AnthropicMessage Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `role` | `string` | **Yes** | Either `"user"` or `"assistant"`. |
| `content` | `string \| object[]` | **Yes** | Text content or array of content blocks. |

---

### GET /health

Lightweight liveness probe. Always returns 200 OK with the service version. Does not check downstream dependencies, so it can never produce a false negative.

**URL**: `/health`

**Method**: `GET`

**Authentication**: Not required

#### Response

**Status**: `200 OK`

```json
{
  "status": "healthy",
  "version": "0.1.0"
}
```

#### Example

```bash
curl https://sunilkumarvalmiki.xyz/health
```

---

### GET /ready

Readiness probe. Pings the PostgreSQL database with `SELECT 1` to confirm connectivity. Returns 200 if the database is reachable, 503 otherwise. Use this endpoint for load balancer health checks that should only route traffic to fully-ready instances.

**URL**: `/ready`

**Method**: `GET`

**Authentication**: Not required

#### Successful Response

**Status**: `200 OK`

```json
{
  "status": "ready"
}
```

#### Failure Response

**Status**: `503 Service Unavailable`

```json
{
  "status": "not_ready",
  "error": "Database unreachable: connection refused"
}
```

#### Example

```bash
curl https://sunilkumarvalmiki.xyz/ready
```

---

### GET /metrics

**Status**: Planned

Prometheus-compatible metrics endpoint. Will expose request counts, latency histograms, provider health, cache hit rates, and token usage.

**URL**: `/metrics`

**Method**: `GET`

**Authentication**: Not required (typically restricted by network policy)

#### Planned Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `openclaw_requests_total` | Counter | Total number of requests, labeled by provider, tier, and status. |
| `openclaw_request_duration_seconds` | Histogram | End-to-end request latency. |
| `openclaw_scoring_duration_seconds` | Histogram | Scoring engine latency. |
| `openclaw_tokens_total` | Counter | Total tokens processed (input + output). |
| `openclaw_cache_hits_total` | Counter | Number of cache hits. |
| `openclaw_cache_misses_total` | Counter | Number of cache misses. |
| `openclaw_provider_health` | Gauge | Provider health status (1 = healthy, 0 = unhealthy). |
| `openclaw_circuit_breaker_state` | Gauge | Circuit breaker state per provider. |

---

## Router Extensions

The OpenClaw Router extends the standard OpenAI API with custom fields for routing control. These fields are stripped from the request before forwarding to upstream providers.

### Cost Profiles

Control how aggressively the router optimizes for cost by setting `x_cost_profile` in the request body.

| Profile | Description | Avg Cost/1M Tokens | Savings vs Premium |
|---------|-------------|--------------------|--------------------|
| `eco` | Cheapest model that meets quality thresholds. | ~$2.05 | ~92% |
| `auto` | Balanced cost/quality trade-off. **Default**. | ~$8.40 | ~67% |
| `premium` | Highest quality model regardless of price. | ~$25.00 | 0% |
| `free` | Free-tier models only (e.g., local Ollama). | $0.00 | 100% |

The `"balanced"` string is also accepted as an alias for `"auto"`.

### Tier Hints

The `x_tier_hint` field lets clients indicate the user's subscription tier, which influences budget allocation and model selection.

| Value | Description |
|-------|-------------|
| `free` | Free-tier user. Lower rate limits, budget-constrained model selection. |
| `pro` | Pro user. Standard limits and model access. |
| `enterprise` | Enterprise user. Highest limits and priority routing. |

### Router Metadata

Every response includes an `x_router_metadata` object with routing telemetry:

| Field | Type | Description |
|-------|------|-------------|
| `provider` | `string` | Provider that served the request (e.g., `"openai"`, `"gemini"`, `"ollama"`). |
| `tier` | `string` | Complexity tier assigned by the scoring engine: `"simple"`, `"medium"`, `"complex"`, or `"reasoning"`. |
| `cost_profile` | `string` | Cost profile used for this request. |
| `cost_usd` | `number \| null` | Estimated cost of this request in USD. |
| `cost_without_router_usd` | `number \| null` | What the request would have cost without routing optimization. |
| `savings_percent` | `number \| null` | Percentage savings achieved by routing. |
| `cache_hit` | `boolean` | Whether the response was served from cache. |
| `latency_ms` | `integer \| null` | Total end-to-end latency in milliseconds. |
| `scoring_ms` | `number \| null` | Time spent in the scoring engine in milliseconds. |

---

## Error Responses

All errors return a consistent JSON structure:

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable error description"
  }
}
```

### Error Codes

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 400 | `BAD_REQUEST` | Invalid request body or missing required fields. |
| 401 | `UNAUTHORIZED` | Missing or invalid API key. |
| 404 | `NOT_FOUND` | Resource or model not found. |
| 429 | `RATE_LIMIT_EXCEEDED` | Requests per minute or tokens per minute limit exceeded. |
| 502 | `PROVIDER_ERROR` | Upstream LLM provider returned an error or is unreachable. |
| 500 | `INTERNAL_ERROR` | Unexpected server error. |
| 500 | `CACHE_ERROR` | Redis cache operation failed (request still succeeds). |
| 500 | `DATABASE_ERROR` | PostgreSQL operation failed. |

### Example Error Response

```bash
curl -X POST https://sunilkumarvalmiki.xyz/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{}'
```

```json
{
  "error": {
    "code": "BAD_REQUEST",
    "message": "Bad request: missing field `messages`"
  }
}
```

---

## Rate Limiting

The gateway implements token-aware rate limiting per API key using a sliding-window token bucket algorithm.

| Limit Type | Default | Description |
|------------|---------|-------------|
| Requests per minute (RPM) | Configurable per key | Maximum number of requests in a 60-second window. |
| Tokens per minute (TPM) | Configurable per key | Maximum estimated tokens in a 60-second window. |

When a rate limit is exceeded, the API returns:

**Status**: `429 Too Many Requests`

```json
{
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "Rate limit exceeded: Requests per minute limit exceeded"
  }
}
```

The token counter is estimated before the request is sent to the provider. After the response is received, the counter is adjusted based on actual usage. This means estimated tokens are deducted optimistically, and any surplus is refunded upon completion.

Nginx also enforces a global rate limit of 60 requests per minute per IP address with a burst capacity of 20 requests.

---

## SDK Compatibility

The `/v1/chat/completions` endpoint is fully compatible with any OpenAI SDK or client. You can use it as a drop-in replacement by changing the base URL:

### Python (OpenAI SDK)

```python
from openai import OpenAI

client = OpenAI(
    api_key="your-api-key",
    base_url="https://sunilkumarvalmiki.xyz/v1"
)

response = client.chat.completions.create(
    model="gpt-4o",
    messages=[{"role": "user", "content": "Hello!"}],
    extra_body={
        "x_cost_profile": "eco"
    }
)

print(response.choices[0].message.content)
```

### Node.js (OpenAI SDK)

```javascript
import OpenAI from 'openai';

const client = new OpenAI({
  apiKey: 'your-api-key',
  baseURL: 'https://sunilkumarvalmiki.xyz/v1',
});

const response = await client.chat.completions.create({
  model: 'gpt-4o',
  messages: [{ role: 'user', content: 'Hello!' }],
});

console.log(response.choices[0].message.content);
```

### Cursor / Claude Code Configuration

Point your coding assistant to the OpenClaw Router by setting the base URL in your tool's configuration:

```
Base URL: https://sunilkumarvalmiki.xyz/v1
API Key: your-api-key
```
