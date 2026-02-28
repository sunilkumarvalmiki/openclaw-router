"use client"

import { useEffect, useState } from "react"
import {
  Activity,
  CheckCircle2,
  Timer,
  PiggyBank,
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
} from "@/lib/api"

export default function DashboardOverview() {
  const [metrics, setMetrics] = useState<{
    totalRequests: number
    successRate: number
    avgLatency: number
    costSavings: number
    requestsTrend: number
    successTrend: number
    latencyTrend: number
    costTrend: number
  } | null>(null)
  const [hourlyData, setHourlyData] = useState<
    { time: string; requests: number; latency: number; errors: number }[]
  >([])
  const [providerDist, setProviderDist] = useState<
    { name: string; value: number; color: string }[]
  >([])
  const [recentRequests, setRecentRequests] = useState<
    {
      id: string
      timestamp: string
      model: string
      provider: string
      tier: string
      latency: number
      tokens: number
      cost: number
      status: string
    }[]
  >([])

  useEffect(() => {
    async function loadData() {
      const [m, h, p, r] = await Promise.all([
        fetchMetrics(),
        fetchHourlyData(24),
        fetchProviderDistribution(),
        fetchRecentRequests(),
      ])
      setMetrics(m)
      setHourlyData(h)
      setProviderDist(p)
      setRecentRequests(r)
    }
    loadData()
  }, [])

  if (!metrics) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-pulse text-gray-400">Loading dashboard...</div>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
          Overview
        </h2>
        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
          Monitor your LLM router in real-time
        </p>
      </div>

      {/* Metric Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <MetricCard
          title="Total Requests"
          value={metrics.totalRequests.toLocaleString()}
          trend={metrics.requestsTrend}
          icon={Activity}
          iconColor="text-blue-600 dark:text-blue-400"
        />
        <MetricCard
          title="Success Rate"
          value={`${metrics.successRate}%`}
          trend={metrics.successTrend}
          icon={CheckCircle2}
          iconColor="text-green-600 dark:text-green-400"
        />
        <MetricCard
          title="Avg Latency"
          value={`${metrics.avgLatency}ms`}
          trend={metrics.latencyTrend}
          icon={Timer}
          iconColor="text-orange-600 dark:text-orange-400"
        />
        <MetricCard
          title="Cost Savings"
          value={`${metrics.costSavings}%`}
          trend={metrics.costTrend}
          icon={PiggyBank}
          iconColor="text-purple-600 dark:text-purple-400"
        />
      </div>

      {/* Charts Row */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Request Volume Chart */}
        <div className="lg:col-span-2 bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-6 shadow-sm">
          <h3 className="text-base font-semibold text-gray-900 dark:text-white mb-4">
            Request Volume (Last 24h)
          </h3>
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={hourlyData}>
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
              />
            </LineChart>
          </ResponsiveContainer>
        </div>

        {/* Provider Distribution Pie */}
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-6 shadow-sm">
          <h3 className="text-base font-semibold text-gray-900 dark:text-white mb-4">
            Provider Distribution
          </h3>
          <ResponsiveContainer width="100%" height={300}>
            <PieChart>
              <Pie
                data={providerDist}
                cx="50%"
                cy="50%"
                innerRadius={60}
                outerRadius={100}
                paddingAngle={2}
                dataKey="value"
              >
                {providerDist.map((entry, index) => (
                  <Cell key={`cell-${index}`} fill={entry.color} />
                ))}
              </Pie>
              <Tooltip
                formatter={(value) => [`${value}%`, "Share"]}
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
        </div>
      </div>

      {/* Recent Requests Table */}
      <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-800">
          <h3 className="text-base font-semibold text-gray-900 dark:text-white">
            Recent Requests
          </h3>
        </div>
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
                    {new Date(req.timestamp).toLocaleTimeString()}
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
                    {req.latency}ms
                  </td>
                  <td className="px-6 py-3 text-sm text-gray-600 dark:text-gray-300 whitespace-nowrap">
                    {req.tokens.toLocaleString()}
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
      </div>
    </div>
  )
}
