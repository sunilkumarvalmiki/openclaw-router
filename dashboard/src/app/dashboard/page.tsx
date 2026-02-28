"use client"

import { useEffect, useState, useCallback } from "react"
import {
  Activity,
  CheckCircle2,
  Timer,
  Zap,
  RefreshCw,
  Clock,
} from "lucide-react"
import {
  LineChart,
  Line,
  PieChart,
  Pie,
  Cell,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from "recharts"
import { MetricCard } from "@/components/MetricCard"
import {
  fetchMetrics,
  fetchHourlyData,
  fetchProviderDistribution,
  fetchRecentRequests,
  formatUptime,
  formatTimestamp,
  type DashboardMetrics,
  type HourlyDataPoint,
  type ProviderDistribution,
  type RecentRequest,
} from "@/lib/api"

const PROVIDER_COLORS: Record<string, string> = {
  ollama: "#6366f1",
  openai: "#10b981",
  gemini: "#f59e0b",
  xai: "#ef4444",
  deepseek: "#3b82f6",
  openrouter: "#8b5cf6",
  bedrock: "#ec4899",
  custom: "#14b8a6",
}

export default function DashboardOverview() {
  const [metrics, setMetrics] = useState<DashboardMetrics | null>(null)
  const [hourlyData, setHourlyData] = useState<HourlyDataPoint[]>([])
  const [providerDist, setProviderDist] = useState<ProviderDistribution[]>([])
  const [recentRequests, setRecentRequests] = useState<RecentRequest[]>([])
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const [lastRefresh, setLastRefresh] = useState<Date>(new Date())

  const loadData = useCallback(async () => {
    try {
      setError(null)
      const [m, h, p, r] = await Promise.all([
        fetchMetrics(),
        fetchHourlyData(),
        fetchProviderDistribution(),
        fetchRecentRequests(),
      ])
      setMetrics(m)
      setHourlyData(h)
      setProviderDist(p)
      setRecentRequests(r)
      setLastRefresh(new Date())
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
    const interval = setInterval(loadData, 10000) // Refresh every 10 seconds
    return () => clearInterval(interval)
  }, [loadData])

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="flex items-center gap-3 text-gray-400">
          <RefreshCw className="w-5 h-5 animate-spin" />
          <span>Connecting to gateway...</span>
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

  const successRate =
    metrics.total_requests > 0
      ? ((metrics.successful_requests / metrics.total_requests) * 100).toFixed(1)
      : "0.0"

  const chartData = hourlyData.map((d) => ({
    time: new Date(d.timestamp * 1000).toLocaleTimeString("en-US", {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    }),
    requests: d.requests,
    errors: d.errors,
  }))

  const pieData = providerDist.map((p) => ({
    name: p.name,
    value: p.requests,
    color: PROVIDER_COLORS[p.name] || "#94a3b8",
  }))

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
            Overview
          </h2>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1 flex items-center gap-1">
            <Clock className="w-3.5 h-3.5" />
            Uptime: {formatUptime(metrics.uptime_seconds)} &middot; Last
            refresh: {lastRefresh.toLocaleTimeString()}
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

      {/* Metric Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <MetricCard
          title="Total Requests"
          value={metrics.total_requests.toLocaleString()}
          trend={0}
          icon={Activity}
          iconColor="text-blue-600 dark:text-blue-400"
        />
        <MetricCard
          title="Success Rate"
          value={`${successRate}%`}
          trend={0}
          icon={CheckCircle2}
          iconColor="text-green-600 dark:text-green-400"
        />
        <MetricCard
          title="Avg Latency"
          value={`${Math.round(metrics.avg_latency_ms)}ms`}
          trend={0}
          icon={Timer}
          iconColor="text-orange-600 dark:text-orange-400"
        />
        <MetricCard
          title="Total Tokens"
          value={metrics.total_tokens.toLocaleString()}
          trend={0}
          icon={Zap}
          iconColor="text-purple-600 dark:text-purple-400"
        />
      </div>

      {/* Charts Row */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Request Volume Chart */}
        <div className="lg:col-span-2 bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-6 shadow-sm">
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
                <Line
                  type="monotone"
                  dataKey="requests"
                  stroke="#3b82f6"
                  strokeWidth={2}
                  dot={false}
                  activeDot={{ r: 4 }}
                  name="Requests"
                />
                <Line
                  type="monotone"
                  dataKey="errors"
                  stroke="#ef4444"
                  strokeWidth={2}
                  dot={false}
                  activeDot={{ r: 4 }}
                  name="Errors"
                />
                <Legend />
              </LineChart>
            </ResponsiveContainer>
          ) : (
            <div className="flex items-center justify-center h-[300px] text-gray-400">
              No request data yet. Send some requests to see charts.
            </div>
          )}
        </div>

        {/* Provider Distribution Pie */}
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-6 shadow-sm">
          <h3 className="text-base font-semibold text-gray-900 dark:text-white mb-4">
            Provider Distribution
          </h3>
          {pieData.length > 0 && pieData.some((p) => p.value > 0) ? (
            <ResponsiveContainer width="100%" height={300}>
              <PieChart>
                <Pie
                  data={pieData}
                  cx="50%"
                  cy="50%"
                  innerRadius={60}
                  outerRadius={100}
                  paddingAngle={2}
                  dataKey="value"
                >
                  {pieData.map((entry, index) => (
                    <Cell key={`cell-${index}`} fill={entry.color} />
                  ))}
                </Pie>
                <Tooltip
                  formatter={(value) => [
                    `${value} requests`,
                    "Count",
                  ]}
                  contentStyle={{
                    backgroundColor: "rgba(255,255,255,0.95)",
                    border: "1px solid #e5e7eb",
                    borderRadius: "12px",
                  }}
                />
                <Legend
                  verticalAlign="bottom"
                  height={36}
                  iconSize={8}
                  iconType="circle"
                />
              </PieChart>
            </ResponsiveContainer>
          ) : (
            <div className="flex items-center justify-center h-[300px] text-gray-400">
              No provider data yet.
            </div>
          )}
        </div>
      </div>

      {/* Recent Requests Table */}
      <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-800">
          <h3 className="text-base font-semibold text-gray-900 dark:text-white">
            Recent Requests
          </h3>
        </div>
        {recentRequests.length > 0 ? (
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-gray-100 dark:border-gray-800">
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Time
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Model
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Provider
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Tier
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Latency
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Tokens
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Status
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100 dark:divide-gray-800">
                {recentRequests.map((req) => (
                  <tr
                    key={req.id}
                    className="hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
                  >
                    <td className="px-6 py-3 text-sm text-gray-500 dark:text-gray-400 whitespace-nowrap">
                      {formatTimestamp(req.timestamp)}
                    </td>
                    <td className="px-6 py-3 text-sm font-medium text-gray-900 dark:text-white whitespace-nowrap">
                      {req.model}
                    </td>
                    <td className="px-6 py-3 text-sm text-gray-600 dark:text-gray-300 whitespace-nowrap">
                      {req.provider}
                    </td>
                    <td className="px-6 py-3 whitespace-nowrap">
                      <span
                        className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${
                          req.tier === "Simple"
                            ? "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400"
                            : req.tier === "Medium"
                            ? "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400"
                            : req.tier === "Complex"
                            ? "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400"
                            : "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400"
                        }`}
                      >
                        {req.tier}
                      </span>
                    </td>
                    <td className="px-6 py-3 text-sm text-gray-600 dark:text-gray-300 whitespace-nowrap">
                      {req.latency_ms}ms
                    </td>
                    <td className="px-6 py-3 text-sm text-gray-600 dark:text-gray-300 whitespace-nowrap">
                      {req.total_tokens.toLocaleString()}
                    </td>
                    <td className="px-6 py-3 whitespace-nowrap">
                      <span
                        className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${
                          req.status === "success"
                            ? "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400"
                            : "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400"
                        }`}
                      >
                        {req.status}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="flex items-center justify-center h-32 text-gray-400">
            No requests yet. Send requests through the router to see them here.
          </div>
        )}
      </div>
    </div>
  )
}
