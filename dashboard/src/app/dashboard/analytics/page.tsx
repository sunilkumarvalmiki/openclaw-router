"use client"

import { useEffect, useState } from "react"
import {
  LineChart,
  Line,
  BarChart,
  Bar,
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
import {
  fetchHourlyData,
  fetchDailyData,
  fetchTierDistribution,
  fetchLatencyDistribution,
  fetchRequestsByProvider,
} from "@/lib/api"

type TimeRange = "24h" | "7d" | "30d"

export default function AnalyticsPage() {
  const [timeRange, setTimeRange] = useState<TimeRange>("24h")
  const [hourlyData, setHourlyData] = useState<
    { time: string; requests: number; latency: number; errors: number }[]
  >([])
  const [dailyData, setDailyData] = useState<
    {
      date: string
      requests: number
      latency: number
      errors: number
      tokens: number
      cost: number
    }[]
  >([])
  const [tierDist, setTierDist] = useState<
    { name: string; value: number; color: string }[]
  >([])
  const [latencyDist, setLatencyDist] = useState<
    { label: string; value: number }[]
  >([])
  const [requestsByProvider, setRequestsByProvider] = useState<
    {
      provider: string
      simple: number
      medium: number
      complex: number
      reasoning: number
    }[]
  >([])

  useEffect(() => {
    async function loadData() {
      const [h, d, t, l, r] = await Promise.all([
        fetchHourlyData(24),
        fetchDailyData(30),
        fetchTierDistribution(),
        fetchLatencyDistribution(),
        fetchRequestsByProvider(),
      ])
      setHourlyData(h)
      setDailyData(d)
      setTierDist(t)
      setLatencyDist(l)
      setRequestsByProvider(r)
    }
    loadData()
  }, [])

  const volumeData =
    timeRange === "24h"
      ? hourlyData
      : timeRange === "7d"
      ? dailyData.slice(-7)
      : dailyData

  const volumeXKey = timeRange === "24h" ? "time" : "date"

  const tokenData =
    timeRange === "24h"
      ? hourlyData.map((d) => ({
          ...d,
          tokens: Math.floor(Math.random() * 50000) + 10000,
        }))
      : timeRange === "7d"
      ? dailyData.slice(-7)
      : dailyData

  return (
    <div className="space-y-6">
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
            Analytics
          </h2>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Detailed performance and usage analytics
          </p>
        </div>

        {/* Time Range Selector */}
        <div className="flex bg-gray-100 dark:bg-gray-800 rounded-xl p-1">
          {(["24h", "7d", "30d"] as TimeRange[]).map((range) => (
            <button
              key={range}
              onClick={() => setTimeRange(range)}
              className={`px-4 py-2 text-sm font-medium rounded-lg transition-colors ${
                timeRange === range
                  ? "bg-white dark:bg-gray-700 text-gray-900 dark:text-white shadow-sm"
                  : "text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300"
              }`}
            >
              {range}
            </button>
          ))}
        </div>
      </div>

      {/* Request Volume Over Time */}
      <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-6 shadow-sm">
        <h3 className="text-base font-semibold text-gray-900 dark:text-white mb-4">
          Request Volume Over Time
        </h3>
        <ResponsiveContainer width="100%" height={300}>
          <LineChart data={volumeData}>
            <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
            <XAxis
              dataKey={volumeXKey}
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
      </div>

      {/* Row: Requests by Provider + Tier Distribution */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Requests by Provider (Stacked Bar) */}
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-6 shadow-sm">
          <h3 className="text-base font-semibold text-gray-900 dark:text-white mb-4">
            Requests by Provider
          </h3>
          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={requestsByProvider}>
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
              <XAxis
                dataKey="provider"
                tick={{ fontSize: 12 }}
                stroke="#9ca3af"
              />
              <YAxis tick={{ fontSize: 12 }} stroke="#9ca3af" />
              <Tooltip
                contentStyle={{
                  backgroundColor: "rgba(255,255,255,0.95)",
                  border: "1px solid #e5e7eb",
                  borderRadius: "12px",
                }}
              />
              <Legend />
              <Bar
                dataKey="simple"
                name="Simple"
                stackId="a"
                fill="#10b981"
                radius={[0, 0, 0, 0]}
              />
              <Bar
                dataKey="medium"
                name="Medium"
                stackId="a"
                fill="#3b82f6"
              />
              <Bar
                dataKey="complex"
                name="Complex"
                stackId="a"
                fill="#f59e0b"
              />
              <Bar
                dataKey="reasoning"
                name="Reasoning"
                stackId="a"
                fill="#ef4444"
                radius={[4, 4, 0, 0]}
              />
            </BarChart>
          </ResponsiveContainer>
        </div>

        {/* Tier Distribution */}
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-6 shadow-sm">
          <h3 className="text-base font-semibold text-gray-900 dark:text-white mb-4">
            Tier Distribution
          </h3>
          <ResponsiveContainer width="100%" height={300}>
            <PieChart>
              <Pie
                data={tierDist}
                cx="50%"
                cy="50%"
                innerRadius={60}
                outerRadius={110}
                paddingAngle={3}
                dataKey="value"
                label={({ name, value }) => `${name}: ${value}%`}
              >
                {tierDist.map((entry, index) => (
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
            </PieChart>
          </ResponsiveContainer>
        </div>
      </div>

      {/* Row: Latency Distribution + Token Usage */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Latency Distribution */}
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-6 shadow-sm">
          <h3 className="text-base font-semibold text-gray-900 dark:text-white mb-4">
            Latency Distribution
          </h3>
          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={latencyDist}>
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
              <XAxis
                dataKey="label"
                tick={{ fontSize: 12 }}
                stroke="#9ca3af"
              />
              <YAxis
                tick={{ fontSize: 12 }}
                stroke="#9ca3af"
                label={{
                  value: "ms",
                  position: "insideLeft",
                  offset: -5,
                  style: { fontSize: 12, fill: "#9ca3af" },
                }}
              />
              <Tooltip
                formatter={(value) => [`${value}ms`, "Latency"]}
                contentStyle={{
                  backgroundColor: "rgba(255,255,255,0.95)",
                  border: "1px solid #e5e7eb",
                  borderRadius: "12px",
                }}
              />
              <Bar dataKey="value" fill="#f59e0b" radius={[6, 6, 0, 0]}>
                {latencyDist.map((_, index) => (
                  <Cell
                    key={`cell-${index}`}
                    fill={
                      ["#10b981", "#3b82f6", "#f59e0b", "#ef4444", "#dc2626"][
                        index
                      ]
                    }
                  />
                ))}
              </Bar>
            </BarChart>
          </ResponsiveContainer>
        </div>

        {/* Token Usage Over Time */}
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-6 shadow-sm">
          <h3 className="text-base font-semibold text-gray-900 dark:text-white mb-4">
            Token Usage Over Time
          </h3>
          <ResponsiveContainer width="100%" height={300}>
            <AreaChart data={tokenData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
              <XAxis
                dataKey={volumeXKey}
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
        </div>
      </div>
    </div>
  )
}
