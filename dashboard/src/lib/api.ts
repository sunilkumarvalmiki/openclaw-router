// API client for the OpenClaw Router gateway
// Fetches real-time data from the gateway API endpoints

const GATEWAY_URL =
  process.env.NEXT_PUBLIC_GATEWAY_URL || "http://localhost:8080"

// ---------------------------------------------------------------------------
// Types matching gateway API responses
// ---------------------------------------------------------------------------

export interface DashboardMetrics {
  total_requests: number
  successful_requests: number
  failed_requests: number
  avg_latency_ms: number
  cache_hits: number
  cache_misses: number
  uptime_seconds: number
  total_tokens: number
  total_prompt_tokens: number
  total_completion_tokens: number
}

export interface ProviderMetrics {
  name: string
  is_healthy: boolean
  total_requests: number
  successful_requests: number
  failed_requests: number
  avg_latency_ms: number
  error_rate: number
  total_tokens: number
}

export interface RecentRequest {
  id: string
  timestamp: number
  model: string
  provider: string
  tier: string
  cost_profile: string
  status: string
  latency_ms: number
  prompt_tokens: number
  completion_tokens: number
  total_tokens: number
  cache_hit: boolean
  scoring_ms: number
}

export interface TierCount {
  name: string
  count: number
}

export interface ProviderDistribution {
  name: string
  requests: number
}

export interface HourlyDataPoint {
  timestamp: number
  requests: number
  errors: number
  avg_latency_ms: number
  tokens: number
}

export interface ModelInfo {
  id: string
  provider: string
  is_available: boolean
}

export interface GatewayConfig {
  host: string
  port: number
  default_cost_profile: string
  cache_ttl_seconds: number
  max_cache_entries: number
  pricing_sync_interval_seconds: number
  has_database: boolean
  dashboard_url: string
  providers: string[]
}

// ---------------------------------------------------------------------------
// Fetch helper — throws on error, no mock fallback
// ---------------------------------------------------------------------------

async function fetchFromGateway<T>(endpoint: string): Promise<T> {
  const res = await fetch(`${GATEWAY_URL}${endpoint}`, {
    cache: "no-store",
  })
  if (!res.ok) {
    throw new Error(`Gateway API error: ${res.status} ${res.statusText}`)
  }
  return await res.json()
}

// ---------------------------------------------------------------------------
// API functions — all fetch real data from gateway
// ---------------------------------------------------------------------------

export async function fetchMetrics(): Promise<DashboardMetrics> {
  return fetchFromGateway<DashboardMetrics>("/api/v1/stats")
}

export async function fetchProviderStats(): Promise<ProviderMetrics[]> {
  return fetchFromGateway<ProviderMetrics[]>("/api/v1/stats/providers")
}

export async function fetchTierDistribution(): Promise<TierCount[]> {
  return fetchFromGateway<TierCount[]>("/api/v1/stats/tiers")
}

export async function fetchProviderDistribution(): Promise<
  ProviderDistribution[]
> {
  return fetchFromGateway<ProviderDistribution[]>(
    "/api/v1/stats/providers/distribution"
  )
}

export async function fetchHourlyData(): Promise<HourlyDataPoint[]> {
  return fetchFromGateway<HourlyDataPoint[]>("/api/v1/stats/hourly")
}

export async function fetchRecentRequests(): Promise<RecentRequest[]> {
  return fetchFromGateway<RecentRequest[]>("/api/v1/recent-requests")
}

export async function fetchModels(): Promise<ModelInfo[]> {
  return fetchFromGateway<ModelInfo[]>("/api/v1/models")
}

export async function fetchConfig(): Promise<GatewayConfig> {
  return fetchFromGateway<GatewayConfig>("/api/v1/config")
}

// ---------------------------------------------------------------------------
// Currency utilities
// ---------------------------------------------------------------------------

const exchangeRates: Record<string, number> = {
  USD: 1,
  INR: 83.5,
  EUR: 0.92,
}

export function convertCurrency(
  amountUSD: number,
  currency: string
): number {
  return amountUSD * (exchangeRates[currency] || 1)
}

export function formatCurrency(amount: number, currency: string): string {
  if (currency === "INR") {
    const formatted = formatIndianNumber(amount)
    return `₹${formatted}`
  }
  if (currency === "EUR") {
    return `€${amount.toLocaleString("en-US", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    })}`
  }
  return `$${amount.toLocaleString("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })}`
}

function formatIndianNumber(num: number): string {
  const parts = num.toFixed(2).split(".")
  const intPart = parts[0]
  const decPart = parts[1]

  const lastThree = intPart.slice(-3)
  const otherDigits = intPart.slice(0, -3)

  let formatted = lastThree
  if (otherDigits.length > 0) {
    const reversed = otherDigits.split("").reverse().join("")
    const groups = reversed.match(/.{1,2}/g) || []
    const withCommas = groups.join(",").split("").reverse().join("")
    formatted = `${withCommas},${lastThree}`
  }

  return `${formatted}.${decPart}`
}

// ---------------------------------------------------------------------------
// Helper: format uptime
// ---------------------------------------------------------------------------

export function formatUptime(seconds: number): string {
  const days = Math.floor(seconds / 86400)
  const hours = Math.floor((seconds % 86400) / 3600)
  const mins = Math.floor((seconds % 3600) / 60)
  if (days > 0) return `${days}d ${hours}h ${mins}m`
  if (hours > 0) return `${hours}h ${mins}m`
  return `${mins}m`
}

// ---------------------------------------------------------------------------
// Helper: format timestamp
// ---------------------------------------------------------------------------

export function formatTimestamp(epochSeconds: number): string {
  return new Date(epochSeconds * 1000).toLocaleString()
}
