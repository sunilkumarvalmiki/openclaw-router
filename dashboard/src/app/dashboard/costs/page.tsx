"use client"

import { useEffect, useState, useCallback } from "react"
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Cell,
} from "recharts"
import { Zap, ArrowRightLeft, RefreshCw } from "lucide-react"
import {
  fetchMetrics,
  fetchProviderStats,
  convertCurrency,
  formatCurrency,
  type DashboardMetrics,
  type ProviderMetrics,
} from "@/lib/api"

type Currency = "INR" | "USD" | "EUR"

const PROVIDER_COLORS: Record<string, string> = {
  ollama: "#6366f1",
  openai: "#10b981",
  "github-copilot": "#1f6feb",
  gemini: "#f59e0b",
  xai: "#ef4444",
  deepseek: "#3b82f6",
  openrouter: "#8b5cf6",
  bedrock: "#ec4899",
  custom: "#14b8a6",
}

export default function CostsPage() {
  const [currency, setCurrency] = useState<Currency>("INR")
  const [metrics, setMetrics] = useState<DashboardMetrics | null>(null)
  const [providers, setProviders] = useState<ProviderMetrics[]>([])
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    const saved = localStorage.getItem("preferred-currency")
    if (saved && ["INR", "USD", "EUR"].includes(saved)) {
      setCurrency(saved as Currency)
    }
  }, [])

  const loadData = useCallback(async () => {
    try {
      setError(null)
      const [m, p] = await Promise.all([fetchMetrics(), fetchProviderStats()])
      setMetrics(m)
      setProviders(p)
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to connect to gateway"
      )
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    loadData()
    const interval = setInterval(loadData, 15000)
    return () => clearInterval(interval)
  }, [loadData])

  const handleCurrencyChange = (c: Currency) => {
    setCurrency(c)
    localStorage.setItem("preferred-currency", c)
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="flex items-center gap-3 text-gray-400">
          <RefreshCw className="w-5 h-5 animate-spin" />
          <span>Loading cost data...</span>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-64 gap-4">
        <div className="text-red-500 text-center">
          <p className="font-semibold">Gateway Unavailable</p>
          <p className="text-sm mt-1">{error}</p>
        </div>
        <button
          onClick={loadData}
          className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors flex items-center gap-2"
        >
          <RefreshCw className="w-4 h-4" />
          Retry
        </button>
      </div>
    )
  }

  if (!metrics) return null

  const tokensByProvider = providers
    .filter((p) => p.total_tokens > 0)
    .map((p) => ({
      name: p.name,
      tokens: p.total_tokens,
      requests: p.total_requests,
      color: PROVIDER_COLORS[p.name] || "#94a3b8",
    }))
    .sort((a, b) => b.tokens - a.tokens)

  const requestsByProvider = providers
    .filter((p) => p.total_requests > 0)
    .map((p) => ({
      name: p.name,
      requests: p.total_requests,
      color: PROVIDER_COLORS[p.name] || "#94a3b8",
    }))
    .sort((a, b) => b.requests - a.requests)

  return (
    <div className="space-y-6">
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
            Cost & Usage
          </h2>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Token usage and cost analytics from real gateway data
          </p>
        </div>

        {/* Currency Toggle */}
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-1.5 text-xs text-gray-400 dark:text-gray-500">
            <ArrowRightLeft className="w-3.5 h-3.5" />
            <span>
              1 USD ={" "}
              {currency === "INR"
                ? "₹83.50"
                : currency === "EUR"
                ? "€0.92"
                : "$1.00"}
            </span>
          </div>
          <div className="flex bg-gray-100 dark:bg-gray-800 rounded-xl p-1">
            {(["INR", "USD", "EUR"] as Currency[]).map((c) => (
              <button
                key={c}
                onClick={() => handleCurrencyChange(c)}
                className={`px-3 py-1.5 text-sm font-medium rounded-lg transition-colors ${
                  currency === c
                    ? "bg-white dark:bg-gray-700 text-gray-900 dark:text-white shadow-sm"
                    : "text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300"
                }`}
              >
                {c === "INR" ? "₹ INR" : c === "EUR" ? "€ EUR" : "$ USD"}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Usage Summary */}
      <div className="bg-gradient-to-br from-indigo-500 to-purple-600 rounded-2xl p-8 shadow-lg text-white">
        <div className="flex items-start justify-between">
          <div>
            <p className="text-indigo-100 text-sm font-medium">
              Total Tokens Processed
            </p>
            <p className="text-4xl font-bold mt-2">
              {metrics.total_tokens.toLocaleString()}
            </p>
            <div className="flex gap-6 mt-3 text-sm text-indigo-100">
              <span>
                Prompt: {metrics.total_prompt_tokens.toLocaleString()}
              </span>
              <span>
                Completion: {metrics.total_completion_tokens.toLocaleString()}
              </span>
            </div>
          </div>
          <div className="p-4 bg-white/10 rounded-2xl">
            <Zap className="w-8 h-8" />
          </div>
        </div>
      </div>

      {/* Usage Stats */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-5 shadow-sm">
          <p className="text-xs text-gray-500 dark:text-gray-400">
            Total Requests
          </p>
          <p className="text-2xl font-bold text-gray-900 dark:text-white mt-1">
            {metrics.total_requests.toLocaleString()}
          </p>
        </div>
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-5 shadow-sm">
          <p className="text-xs text-gray-500 dark:text-gray-400">
            Avg Tokens / Request
          </p>
          <p className="text-2xl font-bold text-gray-900 dark:text-white mt-1">
            {metrics.total_requests > 0
              ? Math.round(
                  metrics.total_tokens / metrics.total_requests
                ).toLocaleString()
              : "0"}
          </p>
        </div>
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-5 shadow-sm">
          <p className="text-xs text-gray-500 dark:text-gray-400">
            Active Providers
          </p>
          <p className="text-2xl font-bold text-gray-900 dark:text-white mt-1">
            {providers.filter((p) => p.total_requests > 0).length}
          </p>
        </div>
      </div>

      {/* Charts Row */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Tokens by Provider */}
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-6 shadow-sm">
          <h3 className="text-base font-semibold text-gray-900 dark:text-white mb-4">
            Tokens by Provider
          </h3>
          {tokensByProvider.length > 0 ? (
            <ResponsiveContainer width="100%" height={300}>
              <BarChart data={tokensByProvider} layout="vertical">
                <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
                <XAxis
                  type="number"
                  tick={{ fontSize: 12 }}
                  stroke="#9ca3af"
                  tickFormatter={(v) => v.toLocaleString()}
                />
                <YAxis
                  type="category"
                  dataKey="name"
                  tick={{ fontSize: 12 }}
                  stroke="#9ca3af"
                  width={80}
                />
                <Tooltip
                  formatter={(value) => [
                    (value as number).toLocaleString(),
                    "Tokens",
                  ]}
                  contentStyle={{
                    backgroundColor: "rgba(255,255,255,0.95)",
                    border: "1px solid #e5e7eb",
                    borderRadius: "12px",
                  }}
                />
                <Bar dataKey="tokens" radius={[0, 6, 6, 0]}>
                  {tokensByProvider.map((entry, index) => (
                    <Cell key={`cell-${index}`} fill={entry.color} />
                  ))}
                </Bar>
              </BarChart>
            </ResponsiveContainer>
          ) : (
            <div className="flex items-center justify-center h-[300px] text-gray-400">
              No token data yet.
            </div>
          )}
        </div>

        {/* Requests by Provider */}
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-6 shadow-sm">
          <h3 className="text-base font-semibold text-gray-900 dark:text-white mb-4">
            Requests by Provider
          </h3>
          {requestsByProvider.length > 0 ? (
            <ResponsiveContainer width="100%" height={300}>
              <BarChart data={requestsByProvider} layout="vertical">
                <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
                <XAxis
                  type="number"
                  tick={{ fontSize: 12 }}
                  stroke="#9ca3af"
                />
                <YAxis
                  type="category"
                  dataKey="name"
                  tick={{ fontSize: 12 }}
                  stroke="#9ca3af"
                  width={80}
                />
                <Tooltip
                  formatter={(value) => [
                    (value as number).toLocaleString(),
                    "Requests",
                  ]}
                  contentStyle={{
                    backgroundColor: "rgba(255,255,255,0.95)",
                    border: "1px solid #e5e7eb",
                    borderRadius: "12px",
                  }}
                />
                <Bar dataKey="requests" radius={[0, 6, 6, 0]}>
                  {requestsByProvider.map((entry, index) => (
                    <Cell key={`cell-${index}`} fill={entry.color} />
                  ))}
                </Bar>
              </BarChart>
            </ResponsiveContainer>
          ) : (
            <div className="flex items-center justify-center h-[300px] text-gray-400">
              No request data yet.
            </div>
          )}
        </div>
      </div>

      {/* Note */}
      <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-2xl p-4">
        <p className="text-sm text-blue-800 dark:text-blue-300">
          Cost estimation requires pricing data from providers. Currently showing token usage metrics.
          Pricing integration will be available when the pricing sync service is configured.
        </p>
      </div>
    </div>
  )
}
