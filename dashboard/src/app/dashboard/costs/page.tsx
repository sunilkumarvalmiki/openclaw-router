"use client"

import { useEffect, useState } from "react"
import {
  LineChart,
  Line,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
  Cell,
} from "recharts"
import { PiggyBank, TrendingDown, ArrowRightLeft } from "lucide-react"
import { fetchCostData, convertCurrency, formatCurrency } from "@/lib/api"

type Currency = "INR" | "USD" | "EUR"

export default function CostsPage() {
  const [currency, setCurrency] = useState<Currency>("INR")
  const [costData, setCostData] = useState<{
    monthlySavings: number
    monthlyWithRouter: number
    monthlyWithoutRouter: number
    dailyCosts: {
      date: string
      withRouter: number
      withoutRouter: number
      savings: number
    }[]
    costByProvider: { name: string; cost: number; color: string }[]
    costByModel: { name: string; cost: number; color: string }[]
    savingsByProfile: {
      profile: string
      savings: number
      percentage: number
    }[]
  } | null>(null)

  useEffect(() => {
    // Load currency preference from localStorage
    const saved = localStorage.getItem("preferred-currency")
    if (saved && ["INR", "USD", "EUR"].includes(saved)) {
      setCurrency(saved as Currency)
    }
  }, [])

  useEffect(() => {
    async function loadData() {
      const data = await fetchCostData()
      setCostData(data)
    }
    loadData()
  }, [])

  const handleCurrencyChange = (c: Currency) => {
    setCurrency(c)
    localStorage.setItem("preferred-currency", c)
  }

  const fc = (amountUSD: number) =>
    formatCurrency(convertCurrency(amountUSD, currency), currency)

  if (!costData) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-pulse text-gray-400">Loading cost data...</div>
      </div>
    )
  }

  const exchangeRate = currency === "INR" ? 83.5 : currency === "EUR" ? 0.92 : 1

  return (
    <div className="space-y-6">
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
            Cost Tracker
          </h2>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Monitor spending, track savings, and optimize costs
          </p>
        </div>

        {/* Currency Toggle */}
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-1.5 text-xs text-gray-400 dark:text-gray-500">
            <ArrowRightLeft className="w-3.5 h-3.5" />
            <span>
              1 USD = {currency === "INR" ? "₹83.50" : currency === "EUR" ? "€0.92" : "$1.00"}
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

      {/* Savings Summary Card */}
      <div className="bg-gradient-to-br from-green-500 to-emerald-600 rounded-2xl p-8 shadow-lg text-white">
        <div className="flex items-start justify-between">
          <div>
            <p className="text-green-100 text-sm font-medium">
              You saved this month
            </p>
            <p className="text-4xl font-bold mt-2">
              {fc(costData.monthlySavings)}
            </p>
            <p className="text-green-100 mt-2 text-sm">
              By intelligently routing to cost-effective providers
            </p>
          </div>
          <div className="p-4 bg-white/10 rounded-2xl">
            <PiggyBank className="w-8 h-8" />
          </div>
        </div>
      </div>

      {/* Comparison Table */}
      <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-800">
          <h3 className="text-base font-semibold text-gray-900 dark:text-white">
            Cost Comparison: Router vs Direct
          </h3>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b border-gray-100 dark:border-gray-800">
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Period
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  With Router
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Without Router
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Savings
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100 dark:divide-gray-800">
              <tr>
                <td className="px-6 py-4 text-sm font-medium text-gray-900 dark:text-white">
                  Daily (avg)
                </td>
                <td className="px-6 py-4 text-sm text-gray-600 dark:text-gray-300">
                  {fc(costData.monthlyWithRouter / 30)}
                </td>
                <td className="px-6 py-4 text-sm text-gray-600 dark:text-gray-300">
                  {fc(costData.monthlyWithoutRouter / 30)}
                </td>
                <td className="px-6 py-4 text-sm font-medium text-green-600 dark:text-green-400">
                  <div className="flex items-center gap-1">
                    <TrendingDown className="w-4 h-4" />
                    {fc(costData.monthlySavings / 30)}
                  </div>
                </td>
              </tr>
              <tr>
                <td className="px-6 py-4 text-sm font-medium text-gray-900 dark:text-white">
                  Weekly (avg)
                </td>
                <td className="px-6 py-4 text-sm text-gray-600 dark:text-gray-300">
                  {fc(costData.monthlyWithRouter / 4)}
                </td>
                <td className="px-6 py-4 text-sm text-gray-600 dark:text-gray-300">
                  {fc(costData.monthlyWithoutRouter / 4)}
                </td>
                <td className="px-6 py-4 text-sm font-medium text-green-600 dark:text-green-400">
                  <div className="flex items-center gap-1">
                    <TrendingDown className="w-4 h-4" />
                    {fc(costData.monthlySavings / 4)}
                  </div>
                </td>
              </tr>
              <tr>
                <td className="px-6 py-4 text-sm font-medium text-gray-900 dark:text-white">
                  Monthly
                </td>
                <td className="px-6 py-4 text-sm text-gray-600 dark:text-gray-300">
                  {fc(costData.monthlyWithRouter)}
                </td>
                <td className="px-6 py-4 text-sm text-gray-600 dark:text-gray-300">
                  {fc(costData.monthlyWithoutRouter)}
                </td>
                <td className="px-6 py-4 text-sm font-medium text-green-600 dark:text-green-400">
                  <div className="flex items-center gap-1">
                    <TrendingDown className="w-4 h-4" />
                    {fc(costData.monthlySavings)}
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Savings by Profile */}
      <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-6 shadow-sm">
        <h3 className="text-base font-semibold text-gray-900 dark:text-white mb-4">
          Savings by Cost Profile
        </h3>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          {costData.savingsByProfile.map((profile) => (
            <div
              key={profile.profile}
              className="p-4 rounded-xl border border-gray-100 dark:border-gray-800 bg-gray-50 dark:bg-gray-800/50"
            >
              <div className="flex items-center justify-between mb-2">
                <span className="text-sm font-medium text-gray-900 dark:text-white">
                  {profile.profile}
                </span>
                <span className="text-xs font-medium text-green-600 dark:text-green-400">
                  {profile.percentage}% saved
                </span>
              </div>
              <p className="text-lg font-bold text-gray-900 dark:text-white">
                {fc(profile.savings)}
              </p>
              <div className="mt-2 w-full bg-gray-200 dark:bg-gray-700 rounded-full h-1.5">
                <div
                  className="bg-green-500 h-1.5 rounded-full transition-all"
                  style={{ width: `${profile.percentage}%` }}
                />
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Cost Trend Chart */}
      <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-6 shadow-sm">
        <h3 className="text-base font-semibold text-gray-900 dark:text-white mb-4">
          Daily Cost Trend (Last 30 Days)
        </h3>
        <ResponsiveContainer width="100%" height={300}>
          <LineChart data={costData.dailyCosts}>
            <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
            <XAxis
              dataKey="date"
              tick={{ fontSize: 12 }}
              stroke="#9ca3af"
              interval="preserveStartEnd"
            />
            <YAxis
              tick={{ fontSize: 12 }}
              stroke="#9ca3af"
              tickFormatter={(v) => `$${v}`}
            />
            <Tooltip
              formatter={(value, name) => [
                fc(value as number),
                name === "withRouter"
                  ? "With Router"
                  : name === "withoutRouter"
                  ? "Without Router"
                  : "Savings",
              ]}
              contentStyle={{
                backgroundColor: "rgba(255,255,255,0.95)",
                border: "1px solid #e5e7eb",
                borderRadius: "12px",
                boxShadow: "0 4px 6px -1px rgba(0,0,0,0.1)",
              }}
            />
            <Legend
              formatter={(value) =>
                value === "withRouter"
                  ? "With Router"
                  : value === "withoutRouter"
                  ? "Without Router"
                  : "Savings"
              }
            />
            <Line
              type="monotone"
              dataKey="withRouter"
              stroke="#3b82f6"
              strokeWidth={2}
              dot={false}
            />
            <Line
              type="monotone"
              dataKey="withoutRouter"
              stroke="#ef4444"
              strokeWidth={2}
              dot={false}
              strokeDasharray="5 5"
            />
            <Line
              type="monotone"
              dataKey="savings"
              stroke="#10b981"
              strokeWidth={2}
              dot={false}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>

      {/* Cost by Provider + Cost by Model */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-6 shadow-sm">
          <h3 className="text-base font-semibold text-gray-900 dark:text-white mb-4">
            Cost by Provider
          </h3>
          <ResponsiveContainer width="100%" height={300}>
            <BarChart
              data={costData.costByProvider}
              layout="vertical"
            >
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
              <XAxis
                type="number"
                tick={{ fontSize: 12 }}
                stroke="#9ca3af"
                tickFormatter={(v) => `$${v}`}
              />
              <YAxis
                type="category"
                dataKey="name"
                tick={{ fontSize: 12 }}
                stroke="#9ca3af"
                width={80}
              />
              <Tooltip
                formatter={(value) => [fc(value as number), "Cost"]}
                contentStyle={{
                  backgroundColor: "rgba(255,255,255,0.95)",
                  border: "1px solid #e5e7eb",
                  borderRadius: "12px",
                }}
              />
              <Bar dataKey="cost" radius={[0, 6, 6, 0]}>
                {costData.costByProvider.map((entry, index) => (
                  <Cell key={`cell-${index}`} fill={entry.color} />
                ))}
              </Bar>
            </BarChart>
          </ResponsiveContainer>
        </div>

        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-6 shadow-sm">
          <h3 className="text-base font-semibold text-gray-900 dark:text-white mb-4">
            Cost by Model
          </h3>
          <ResponsiveContainer width="100%" height={300}>
            <BarChart
              data={costData.costByModel}
              layout="vertical"
            >
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
              <XAxis
                type="number"
                tick={{ fontSize: 12 }}
                stroke="#9ca3af"
                tickFormatter={(v) => `$${v}`}
              />
              <YAxis
                type="category"
                dataKey="name"
                tick={{ fontSize: 12 }}
                stroke="#9ca3af"
                width={120}
              />
              <Tooltip
                formatter={(value) => [fc(value as number), "Cost"]}
                contentStyle={{
                  backgroundColor: "rgba(255,255,255,0.95)",
                  border: "1px solid #e5e7eb",
                  borderRadius: "12px",
                }}
              />
              <Bar dataKey="cost" radius={[0, 6, 6, 0]}>
                {costData.costByModel.map((entry, index) => (
                  <Cell key={`cell-${index}`} fill={entry.color} />
                ))}
              </Bar>
            </BarChart>
          </ResponsiveContainer>
        </div>
      </div>
    </div>
  )
}
