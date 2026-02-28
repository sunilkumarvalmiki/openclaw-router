"use client"

import { useState } from "react"
import {
  Database,
  Trash2,
  RefreshCw,
  HardDrive,
  Clock,
  Hash,
  TrendingUp,
} from "lucide-react"

const mockCacheStats = {
  totalEntries: 8432,
  hitRate: 73.5,
  missRate: 26.5,
  memoryUsed: "256 MB",
  memoryTotal: "1 GB",
  avgTtl: 3200,
  evictions: 1245,
}

const mockCacheEntries = [
  {
    key: "prompt:simple:hash_a1b2c3",
    provider: "Groq",
    model: "llama-3.3-70b",
    hits: 142,
    ttl: 3400,
    size: "2.1 KB",
  },
  {
    key: "prompt:medium:hash_d4e5f6",
    provider: "OpenAI",
    model: "gpt-4o-mini",
    hits: 98,
    ttl: 2800,
    size: "4.5 KB",
  },
  {
    key: "prompt:simple:hash_g7h8i9",
    provider: "Groq",
    model: "llama-3.1-8b",
    hits: 87,
    ttl: 3100,
    size: "1.8 KB",
  },
  {
    key: "prompt:complex:hash_j1k2l3",
    provider: "Anthropic",
    model: "claude-3.5-sonnet",
    hits: 56,
    ttl: 1900,
    size: "8.2 KB",
  },
  {
    key: "prompt:reasoning:hash_m4n5o6",
    provider: "Groq",
    model: "deepseek-r1",
    hits: 34,
    ttl: 2400,
    size: "12.5 KB",
  },
  {
    key: "prompt:simple:hash_p7q8r9",
    provider: "Google",
    model: "gemini-2.0-flash",
    hits: 29,
    ttl: 3500,
    size: "1.5 KB",
  },
  {
    key: "prompt:medium:hash_s1t2u3",
    provider: "OpenAI",
    model: "gpt-4o",
    hits: 22,
    ttl: 800,
    size: "6.3 KB",
  },
  {
    key: "prompt:simple:hash_v4w5x6",
    provider: "Groq",
    model: "gemma2-9b",
    hits: 15,
    ttl: 3200,
    size: "1.9 KB",
  },
]

export default function CachePage() {
  const [stats] = useState(mockCacheStats)
  const [entries] = useState(mockCacheEntries)
  const [clearStatus, setClearStatus] = useState<string | null>(null)

  const handleClearAll = () => {
    setClearStatus("Cache cleared successfully!")
    setTimeout(() => setClearStatus(null), 3000)
  }

  const handleRefresh = () => {
    setClearStatus("Cache stats refreshed")
    setTimeout(() => setClearStatus(null), 2000)
  }

  const memoryPercent =
    (parseInt(stats.memoryUsed) /
      parseInt(stats.memoryTotal)) *
    100

  return (
    <div className="space-y-6">
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
            Cache
          </h2>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Semantic cache performance and management
          </p>
        </div>
        <div className="flex gap-3">
          <button
            onClick={handleRefresh}
            className="inline-flex items-center gap-2 px-4 py-2.5 rounded-xl border border-gray-300 dark:border-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 text-sm font-medium transition-colors"
          >
            <RefreshCw className="w-4 h-4" />
            Refresh
          </button>
          <button
            onClick={handleClearAll}
            className="inline-flex items-center gap-2 px-4 py-2.5 rounded-xl bg-red-600 hover:bg-red-700 text-white text-sm font-medium transition-colors"
          >
            <Trash2 className="w-4 h-4" />
            Clear All
          </button>
        </div>
      </div>

      {/* Status Toast */}
      {clearStatus && (
        <div className="fixed top-4 right-4 z-50 px-4 py-3 rounded-xl bg-green-500 text-white text-sm font-medium shadow-lg">
          {clearStatus}
        </div>
      )}

      {/* Stats Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-5 shadow-sm">
          <div className="flex items-center gap-3 mb-3">
            <div className="p-2 rounded-xl bg-blue-50 dark:bg-blue-900/30">
              <Hash className="w-4 h-4 text-blue-600 dark:text-blue-400" />
            </div>
            <span className="text-sm text-gray-500 dark:text-gray-400">
              Total Entries
            </span>
          </div>
          <p className="text-2xl font-bold text-gray-900 dark:text-white">
            {stats.totalEntries.toLocaleString()}
          </p>
        </div>

        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-5 shadow-sm">
          <div className="flex items-center gap-3 mb-3">
            <div className="p-2 rounded-xl bg-green-50 dark:bg-green-900/30">
              <TrendingUp className="w-4 h-4 text-green-600 dark:text-green-400" />
            </div>
            <span className="text-sm text-gray-500 dark:text-gray-400">
              Hit Rate
            </span>
          </div>
          <p className="text-2xl font-bold text-gray-900 dark:text-white">
            {stats.hitRate}%
          </p>
        </div>

        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-5 shadow-sm">
          <div className="flex items-center gap-3 mb-3">
            <div className="p-2 rounded-xl bg-purple-50 dark:bg-purple-900/30">
              <HardDrive className="w-4 h-4 text-purple-600 dark:text-purple-400" />
            </div>
            <span className="text-sm text-gray-500 dark:text-gray-400">
              Memory Used
            </span>
          </div>
          <p className="text-2xl font-bold text-gray-900 dark:text-white">
            {stats.memoryUsed}
          </p>
          <div className="mt-2 w-full bg-gray-200 dark:bg-gray-700 rounded-full h-1.5">
            <div
              className="bg-purple-500 h-1.5 rounded-full"
              style={{ width: `${memoryPercent}%` }}
            />
          </div>
          <p className="text-xs text-gray-400 mt-1">
            of {stats.memoryTotal}
          </p>
        </div>

        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-5 shadow-sm">
          <div className="flex items-center gap-3 mb-3">
            <div className="p-2 rounded-xl bg-orange-50 dark:bg-orange-900/30">
              <Clock className="w-4 h-4 text-orange-600 dark:text-orange-400" />
            </div>
            <span className="text-sm text-gray-500 dark:text-gray-400">
              Evictions
            </span>
          </div>
          <p className="text-2xl font-bold text-gray-900 dark:text-white">
            {stats.evictions.toLocaleString()}
          </p>
        </div>
      </div>

      {/* Cache Entries Table */}
      <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-800 flex items-center gap-2">
          <Database className="w-5 h-5 text-gray-500" />
          <h3 className="text-base font-semibold text-gray-900 dark:text-white">
            Top Cache Entries
          </h3>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b border-gray-100 dark:border-gray-800">
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Cache Key
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Provider
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Model
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Hits
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  TTL
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Size
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100 dark:divide-gray-800">
              {entries.map((entry) => (
                <tr
                  key={entry.key}
                  className="hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
                >
                  <td className="px-6 py-3 text-sm font-mono text-gray-600 dark:text-gray-300 whitespace-nowrap">
                    {entry.key}
                  </td>
                  <td className="px-6 py-3 text-sm text-gray-600 dark:text-gray-300">
                    {entry.provider}
                  </td>
                  <td className="px-6 py-3 text-sm font-medium text-gray-900 dark:text-white">
                    {entry.model}
                  </td>
                  <td className="px-6 py-3 text-sm text-gray-600 dark:text-gray-300">
                    {entry.hits}
                  </td>
                  <td className="px-6 py-3 text-sm text-gray-600 dark:text-gray-300">
                    {entry.ttl}s
                  </td>
                  <td className="px-6 py-3 text-sm text-gray-600 dark:text-gray-300">
                    {entry.size}
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
