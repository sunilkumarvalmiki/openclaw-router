"use client"

import { useEffect, useState, useCallback } from "react"
import {
  LineChart,
  Line,
  PieChart,
  Pie,
  AreaChart,
  Area,
  Cell,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from "recharts"
import { RefreshCw } from "lucide-react"
import {
  fetchHourlyData,
  fetchTierDistribution,
  fetchMetrics,
  fetchProviderStats,
  type HourlyDataPoint,
  type TierCount,
  type DashboardMetrics,
  type ProviderMetrics,
} from "@/lib/api"

const TIER_COLORS: Record<string, string> = {
  Simple: "#10b981",
  Medium: "#3b82f6",
  Complex: "#f59e0b",
  Reasoning: "#ef4444",
}

export default function AnalyticsPage() {
  const [hourlyData, setHourlyData] = useState<HourlyDataPoint[]>([])
  const [tierDist, setTierDist] = useState<TierCount[]>([])
  const [metrics, setMetrics] = useState<DashboardMetrics | null>(null)
  const [providers, setProviders] = useState<ProviderMetrics[]>([])
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)

  const loadData = useCallback(async () => {
    try {
      setError(null)
      const [h, t, m, p] = await Promise.all([
        fetchHourlyData(),
        fetchTierDistribution(),
        fetchMetrics(),
        fetchProviderStats(),
      ])
      setHourlyData(h)
      setTierDist(t)
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

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="flex items-center gap-3 text-gray-400">
          <RefreshCw className="w-5 h-5 animate-spin" />
          <span>Loading analytics...</span>
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

  const chartData = hourlyData.map((d) => ({
    time: new Date(d.timestamp * 1000).toLocaleTimeString("en-US", {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    }),
    requests: d.requests,
    errors: d.errors,
    tokens: d.tokens,
    avg_latency: Math.round(d.avg_latency_ms),
  }))

  const totalTiers = tierDist.reduce((sum, t) => sum + t.count, 0)
  const tierPieData = tierDist.map((t) => ({
    name: t.name,
    value: totalTiers > 0 ? Math.round((t.count / totalTiers) * 100) : 0,
    count: t.count,
    color: TIER_COLORS[t.name] || "#94a3b8",
  }))

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
            Analytics
          </h2>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Real-time performance and usage analytics
          </p>
        </div>
        <button
          onClick={loadData}
          className="p-2 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
          title="Refresh data"
        >
          <RefreshCw className="w-5 h-5" />
        </button>
      </div>

      {/* Summary cards */}
      {metrics && (
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
          <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-4 shadow-sm">
            <p className="text-xs text-gray-500 dark:text-gray-400">
              Total Requests
            </p>
            <p className="text-xl font-bold text-gray-900 dark:text-white mt-1">
              {metrics.total_requests.toLocaleString()}
            </p>
          </div>
          <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-4 shadow-sm">
            <p className="text-xs text-gray-500 dark:text-gray-400">
              Failed Requests
            </p>
            <p className="text-xl font-bold text-red-600 dark:text-red-400 mt-1">
              {metrics.failed_requests.toLocaleString()}
            </p>
          </div>
          <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-4 shadow-sm">
            <p className="text-xs text-gray-500 dark:text-gray-400">
              Total Tokens
            </p>
            <p className="text-xl font-bold text-gray-900 dark:text-white mt-1">
              {metrics.total_tokens.toLocaleString()}
            </p>
          </div>
          <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-4 shadow-sm">
            <p className="text-xs text-gray-500 dark:text-gray-400">
              Cache Hit Rate
            </p>
            <p className="text-xl font-bold text-green-600 dark:text-green-400 mt-1">
              {metrics.cache_hits + metrics.cache_misses > 0
                ? `${(
                    (metrics.cache_hits /
                      (metrics.cache_hits + metrics.cache_misses)) *
                    100
                  ).toFixed(1)}%`
                : "N/A"}
            </p>
          </div>
        </div>
      )}

      {/* Request Volume Over Time */}
      <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-6 shadow-sm">
        <h3 className="text-base font-semibold text-gray-900 dark:text-white mb-4">
          Request Volume (Hourly)
        </h3>
        {chartData.length > 0 ? (
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
              <XAxis
                dataKey="time"
                tick={{ fontSize: 12 }}
                stroke="#9ca3af"
                interval="preserveStartEnd"
              />
              <YAxis tick={{ fontSize: 12 }} stroke="#9ca3af" />
              <Tooltip
                contentStyle={{
                  backgroundColor: "rgba(255,255,255,0.95)",
                  border: "1px solid #e5e7eb",
                  borderRadius: "12px",
                  boxShadow: "0 4px 6px -1px rgba(0,0,0,0.1)",
                }}
              />
              <Legend />
              <Line
                type="monotone"
                dataKey="requests"
                name="Requests"
                stroke="#3b82f6"
                strokeWidth={2}
                dot={false}
                activeDot={{ r: 4 }}
              />
              <Line
                type="monotone"
                dataKey="errors"
                name="Errors"
                stroke="#ef4444"
                strokeWidth={2}
                dot={false}
                activeDot={{ r: 4 }}
              />
            </LineChart>
          </ResponsiveContainer>
        ) : (
          <div className="flex items-center justify-center h-[300px] text-gray-400">
            No hourly data yet. Send requests to see charts.
          </div>
        )}
      </div>

      {/* Row: Provider Stats + Tier Distribution */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Per-provider stats */}
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-6 shadow-sm">
          <h3 className="text-base font-semibold text-gray-900 dark:text-white mb-4">
            Provider Performance
          </h3>
          {providers.length > 0 ? (
            <div className="space-y-3">
              {providers.map((p) => (
                <div
                  key={p.name}
                  className="flex items-center justify-between p-3 rounded-xl bg-gray-50 dark:bg-gray-800/50"
                >
                  <div className="flex items-center gap-3">
                    <span
                      className={`w-2 h-2 rounded-full ${
                        p.is_healthy ? "bg-green-500" : "bg-red-500"
                      }`}
                    />
                    <span className="text-sm font-medium text-gray-900 dark:text-white capitalize">
                      {p.name}
                    </span>
                  </div>
                  <div className="flex items-center gap-4 text-xs text-gray-500 dark:text-gray-400">
                    <span>{p.total_requests} req</span>
                    <span>{Math.round(p.avg_latency_ms)}ms</span>
                    <span
                      className={
                        p.error_rate > 5 ? "text-red-500" : ""
                      }
                    >
                      {p.error_rate.toFixed(1)}% err
                    </span>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="flex items-center justify-center h-[200px] text-gray-400">
              No provider data available.
            </div>
          )}
        </div>

        {/* Tier Distribution */}
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-6 shadow-sm">
          <h3 className="text-base font-semibold text-gray-900 dark:text-white mb-4">
            Tier Distribution
          </h3>
          {tierPieData.some((t) => t.count > 0) ? (
            <ResponsiveContainer width="100%" height={300}>
              <PieChart>
                <Pie
                  data={tierPieData}
                  cx="50%"
                  cy="50%"
                  innerRadius={60}
                  outerRadius={110}
                  paddingAngle={3}
                  dataKey="value"
                  label={({ name, value }) => `${name}: ${value}%`}
                >
                  {tierPieData.map((entry, index) => (
                    <Cell key={`cell-${index}`} fill={entry.color} />
                  ))}
                </Pie>
                <Tooltip
                  formatter={(value, name, entry) => [
                    `${entry.payload.count} requests (${value}%)`,
                    name,
                  ]}
                  contentStyle={{
                    backgroundColor: "rgba(255,255,255,0.95)",
                    border: "1px solid #e5e7eb",
                    borderRadius: "12px",
                  }}
                />
              </PieChart>
            </ResponsiveContainer>
          ) : (
            <div className="flex items-center justify-center h-[300px] text-gray-400">
              No tier data yet.
            </div>
          )}
        </div>
      </div>

      {/* Token Usage Over Time */}
      <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-6 shadow-sm">
        <h3 className="text-base font-semibold text-gray-900 dark:text-white mb-4">
          Token Usage (Hourly)
        </h3>
        {chartData.length > 0 ? (
          <ResponsiveContainer width="100%" height={300}>
            <AreaChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
              <XAxis
                dataKey="time"
                tick={{ fontSize: 12 }}
                stroke="#9ca3af"
                interval="preserveStartEnd"
              />
              <YAxis tick={{ fontSize: 12 }} stroke="#9ca3af" />
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
              <defs>
                <linearGradient
                  id="tokenGradient"
                  x1="0"
                  y1="0"
                  x2="0"
                  y2="1"
                >
                  <stop offset="5%" stopColor="#8b5cf6" stopOpacity={0.3} />
                  <stop offset="95%" stopColor="#8b5cf6" stopOpacity={0} />
                </linearGradient>
              </defs>
              <Area
                type="monotone"
                dataKey="tokens"
                stroke="#8b5cf6"
                strokeWidth={2}
                fill="url(#tokenGradient)"
              />
            </AreaChart>
          </ResponsiveContainer>
        ) : (
          <div className="flex items-center justify-center h-[300px] text-gray-400">
            No token data yet.
          </div>
        )}
      </div>
    </div>
  )
}
