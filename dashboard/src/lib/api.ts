// API client for the OpenClaw Router gateway
// Returns mock data when the gateway is not running

const GATEWAY_URL = process.env.NEXT_PUBLIC_GATEWAY_URL || "http://localhost:8080"

// --- Mock Data ---

function generateHourlyData(hours: number) {
  const data = []
  const now = new Date()
  for (let i = hours; i >= 0; i--) {
    const time = new Date(now.getTime() - i * 60 * 60 * 1000)
    data.push({
      time: time.toLocaleTimeString("en-US", {
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
      }),
      requests: Math.floor(Math.random() * 500) + 100,
      latency: Math.floor(Math.random() * 200) + 50,
      errors: Math.floor(Math.random() * 10),
    })
  }
  return data
}

function generateDailyData(days: number) {
  const data = []
  const now = new Date()
  for (let i = days; i >= 0; i--) {
    const date = new Date(now.getTime() - i * 24 * 60 * 60 * 1000)
    data.push({
      date: date.toLocaleDateString("en-US", {
        month: "short",
        day: "numeric",
      }),
      requests: Math.floor(Math.random() * 5000) + 2000,
      latency: Math.floor(Math.random() * 180) + 60,
      errors: Math.floor(Math.random() * 50) + 5,
      tokens: Math.floor(Math.random() * 500000) + 100000,
      cost: parseFloat((Math.random() * 20 + 5).toFixed(2)),
    })
  }
  return data
}

const mockProviderDistribution = [
  { name: "Groq", value: 35, color: "#3b82f6" },
  { name: "OpenAI", value: 25, color: "#10b981" },
  { name: "Anthropic", value: 20, color: "#f59e0b" },
  { name: "Google", value: 10, color: "#ef4444" },
  { name: "GitHub Copilot", value: 5, color: "#8b5cf6" },
  { name: "Ollama", value: 5, color: "#6366f1" },
]

const mockTierDistribution = [
  { name: "Simple", value: 40, color: "#10b981" },
  { name: "Medium", value: 30, color: "#3b82f6" },
  { name: "Complex", value: 20, color: "#f59e0b" },
  { name: "Reasoning", value: 10, color: "#ef4444" },
]

const mockRecentRequests = [
  {
    id: "req_001",
    timestamp: "2026-03-01T02:15:32Z",
    model: "llama-3.3-70b",
    provider: "Groq",
    tier: "Simple",
    latency: 145,
    tokens: 512,
    cost: 0.0003,
    status: "success",
  },
  {
    id: "req_002",
    timestamp: "2026-03-01T02:14:58Z",
    model: "gpt-4o",
    provider: "OpenAI",
    tier: "Complex",
    latency: 1230,
    tokens: 2048,
    cost: 0.012,
    status: "success",
  },
  {
    id: "req_003",
    timestamp: "2026-03-01T02:14:22Z",
    model: "claude-3.5-sonnet",
    provider: "Anthropic",
    tier: "Reasoning",
    latency: 890,
    tokens: 1024,
    cost: 0.008,
    status: "success",
  },
  {
    id: "req_004",
    timestamp: "2026-03-01T02:13:45Z",
    model: "gemini-2.0-flash",
    provider: "Google",
    tier: "Medium",
    latency: 320,
    tokens: 768,
    cost: 0.002,
    status: "success",
  },
  {
    id: "req_005",
    timestamp: "2026-03-01T02:12:55Z",
    model: "llama-3.3-70b",
    provider: "Groq",
    tier: "Simple",
    latency: 98,
    tokens: 256,
    cost: 0.0001,
    status: "success",
  },
  {
    id: "req_006",
    timestamp: "2026-03-01T02:12:10Z",
    model: "gpt-4o-mini",
    provider: "OpenAI",
    tier: "Simple",
    latency: 450,
    tokens: 512,
    cost: 0.001,
    status: "error",
  },
  {
    id: "req_007",
    timestamp: "2026-03-01T02:11:30Z",
    model: "deepseek-r1",
    provider: "Groq",
    tier: "Reasoning",
    latency: 2100,
    tokens: 4096,
    cost: 0.005,
    status: "success",
  },
  {
    id: "req_008",
    timestamp: "2026-03-01T02:10:45Z",
    model: "claude-3.5-haiku",
    provider: "Anthropic",
    tier: "Medium",
    latency: 210,
    tokens: 384,
    cost: 0.001,
    status: "success",
  },
  {
    id: "req_009",
    timestamp: "2026-03-01T02:09:55Z",
    model: "gemini-2.0-flash",
    provider: "Google",
    tier: "Simple",
    latency: 180,
    tokens: 256,
    cost: 0.0005,
    status: "success",
  },
  {
    id: "req_010",
    timestamp: "2026-03-01T02:09:10Z",
    model: "gpt-4o",
    provider: "OpenAI",
    tier: "Complex",
    latency: 1500,
    tokens: 3072,
    cost: 0.018,
    status: "success",
  },
]

const mockProviders = [
  {
    name: "Groq",
    status: "healthy" as const,
    circuitBreaker: "closed" as const,
    rateLimit: { used: 45, total: 100, unit: "RPM" },
    models: [
      "llama-3.3-70b",
      "llama-3.1-8b",
      "deepseek-r1",
      "gemma2-9b",
      "mixtral-8x7b",
    ],
    avgLatency: 120,
    errorRate: 0.2,
    totalRequests: 15234,
  },
  {
    name: "OpenAI",
    status: "healthy" as const,
    circuitBreaker: "closed" as const,
    rateLimit: { used: 32, total: 60, unit: "RPM" },
    models: ["gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "o1-preview", "o1-mini"],
    avgLatency: 850,
    errorRate: 0.5,
    totalRequests: 12456,
  },
  {
    name: "Anthropic",
    status: "healthy" as const,
    circuitBreaker: "closed" as const,
    rateLimit: { used: 18, total: 50, unit: "RPM" },
    models: ["claude-3.5-sonnet", "claude-3.5-haiku", "claude-3-opus"],
    avgLatency: 650,
    errorRate: 0.3,
    totalRequests: 8932,
  },
  {
    name: "Google",
    status: "degraded" as const,
    circuitBreaker: "half_open" as const,
    rateLimit: { used: 55, total: 60, unit: "RPM" },
    models: ["gemini-2.0-flash", "gemini-1.5-pro", "gemini-1.5-flash"],
    avgLatency: 420,
    errorRate: 5.2,
    totalRequests: 6543,
  },
  {
    name: "GitHub Copilot",
    status: "healthy" as const,
    circuitBreaker: "closed" as const,
    rateLimit: { used: 8, total: 30, unit: "RPM" },
    models: ["copilot-gpt-4o", "copilot-claude-3.5-sonnet"],
    avgLatency: 950,
    errorRate: 1.1,
    totalRequests: 3210,
  },
  {
    name: "Ollama",
    status: "down" as const,
    circuitBreaker: "open" as const,
    rateLimit: { used: 0, total: 100, unit: "RPM" },
    models: ["llama3.2", "codellama", "mistral"],
    avgLatency: 0,
    errorRate: 100,
    totalRequests: 1089,
  },
]

const mockMetrics = {
  totalRequests: 47464,
  successRate: 98.7,
  avgLatency: 342,
  costSavings: 67.3,
  requestsTrend: 12.5,
  successTrend: 0.3,
  latencyTrend: -8.2,
  costTrend: 15.1,
}

const mockCostData = {
  monthlySavings: 2847.5,
  monthlyWithRouter: 1380.25,
  monthlyWithoutRouter: 4227.75,
  dailyCosts: generateDailyData(30).map((d) => ({
    ...d,
    withRouter: parseFloat((d.cost * 0.7).toFixed(2)),
    withoutRouter: d.cost,
    savings: parseFloat((d.cost * 0.3).toFixed(2)),
  })),
  costByProvider: [
    { name: "Groq", cost: 45.2, color: "#3b82f6" },
    { name: "OpenAI", cost: 680.5, color: "#10b981" },
    { name: "Anthropic", cost: 420.3, color: "#f59e0b" },
    { name: "Google", cost: 180.15, color: "#ef4444" },
    { name: "GitHub Copilot", cost: 54.1, color: "#8b5cf6" },
  ],
  costByModel: [
    { name: "gpt-4o", cost: 520.3, color: "#3b82f6" },
    { name: "claude-3.5-sonnet", cost: 380.2, color: "#10b981" },
    { name: "gemini-1.5-pro", cost: 150.8, color: "#f59e0b" },
    { name: "gpt-4o-mini", cost: 95.5, color: "#ef4444" },
    { name: "llama-3.3-70b", cost: 42.1, color: "#8b5cf6" },
    { name: "deepseek-r1", cost: 35.0, color: "#6366f1" },
    { name: "claude-3.5-haiku", cost: 30.5, color: "#ec4899" },
    { name: "gemini-2.0-flash", cost: 25.1, color: "#14b8a6" },
  ],
  savingsByProfile: [
    { profile: "ECO", savings: 1520.3, percentage: 72 },
    { profile: "AUTO", savings: 847.5, percentage: 45 },
    { profile: "PREMIUM", savings: 280.2, percentage: 18 },
    { profile: "FREE", savings: 199.5, percentage: 100 },
  ],
}

const mockLatencyDistribution = [
  { label: "P50", value: 180 },
  { label: "P75", value: 320 },
  { label: "P90", value: 650 },
  { label: "P95", value: 1100 },
  { label: "P99", value: 2400 },
]

const mockRequestsByProvider = [
  {
    provider: "Groq",
    simple: 8500,
    medium: 3200,
    complex: 2100,
    reasoning: 1434,
  },
  {
    provider: "OpenAI",
    simple: 3200,
    medium: 4500,
    complex: 3000,
    reasoning: 1756,
  },
  {
    provider: "Anthropic",
    simple: 1200,
    medium: 2800,
    complex: 3200,
    reasoning: 1732,
  },
  {
    provider: "Google",
    simple: 2100,
    medium: 2400,
    complex: 1200,
    reasoning: 843,
  },
  {
    provider: "Copilot",
    simple: 800,
    medium: 1200,
    complex: 900,
    reasoning: 310,
  },
]

// --- API Functions ---

async function fetchFromGateway<T>(
  endpoint: string,
  fallback: T
): Promise<T> {
  try {
    const res = await fetch(`${GATEWAY_URL}${endpoint}`, {
      cache: "no-store",
    })
    if (!res.ok) throw new Error(`HTTP ${res.status}`)
    return await res.json()
  } catch {
    // Return mock data when gateway is unreachable
    return fallback
  }
}

export async function fetchMetrics() {
  return fetchFromGateway("/api/metrics", mockMetrics)
}

export async function fetchHourlyData(hours = 24) {
  return fetchFromGateway(
    `/api/metrics/hourly?hours=${hours}`,
    generateHourlyData(hours)
  )
}

export async function fetchDailyData(days = 30) {
  return fetchFromGateway(
    `/api/metrics/daily?days=${days}`,
    generateDailyData(days)
  )
}

export async function fetchProviderDistribution() {
  return fetchFromGateway(
    "/api/metrics/providers",
    mockProviderDistribution
  )
}

export async function fetchTierDistribution() {
  return fetchFromGateway("/api/metrics/tiers", mockTierDistribution)
}

export async function fetchRecentRequests() {
  return fetchFromGateway("/api/requests/recent", mockRecentRequests)
}

export async function fetchProviders() {
  return fetchFromGateway("/api/providers", mockProviders)
}

export async function fetchCostData() {
  return fetchFromGateway("/api/costs", mockCostData)
}

export async function fetchLatencyDistribution() {
  return fetchFromGateway(
    "/api/metrics/latency",
    mockLatencyDistribution
  )
}

export async function fetchRequestsByProvider() {
  return fetchFromGateway(
    "/api/metrics/requests-by-provider",
    mockRequestsByProvider
  )
}

// Currency formatting
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

export function formatCurrency(
  amount: number,
  currency: string
): string {
  if (currency === "INR") {
    // Indian comma formatting: 1,00,000
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

  // Indian number system: last 3 digits, then groups of 2
  const lastThree = intPart.slice(-3)
  const otherDigits = intPart.slice(0, -3)

  let formatted = lastThree
  if (otherDigits.length > 0) {
    // Insert commas every 2 digits from right
    const reversed = otherDigits.split("").reverse().join("")
    const groups = reversed.match(/.{1,2}/g) || []
    const withCommas = groups.join(",").split("").reverse().join("")
    formatted = `${withCommas},${lastThree}`
  }

  return `${formatted}.${decPart}`
}
