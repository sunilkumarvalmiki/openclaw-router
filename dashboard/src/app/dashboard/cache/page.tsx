"use client"

import { useEffect, useState, useCallback } from "react"
import {
  Database,
  RefreshCw,
  Hash,
  TrendingUp,
  Zap,
} from "lucide-react"
import {
  fetchMetrics,
  fetchConfig,
  type DashboardMetrics,
  type GatewayConfig,
} from "@/lib/api"

export default function CachePage() {
  const [metrics, setMetrics] = useState<DashboardMetrics | null>(null)
  const [config, setConfig] = useState<GatewayConfig | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)

  const loadData = useCallback(async () => {
    try {
      setError(null)
      const [m, c] = await Promise.all([fetchMetrics(), fetchConfig()])
      setMetrics(m)
      setConfig(c)
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
    const interval = setInterval(loadData, 10000)
    return () => clearInterval(interval)
  }, [loadData])

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="flex items-center gap-3 text-gray-400">
          <RefreshCw className="w-5 h-5 animate-spin" />
          <span>Loading cache stats...</span>
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

  const totalCacheOps = metrics.cache_hits + metrics.cache_misses
  const hitRate =
    totalCacheOps > 0
      ? ((metrics.cache_hits / totalCacheOps) * 100).toFixed(1)
      : "0.0"

  return (
    <div className="space-y-6">
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
            Cache
          </h2>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Semantic cache performance metrics
          </p>
        </div>
        <button
          onClick={loadData}
          className="inline-flex items-center gap-2 px-4 py-2.5 rounded-xl border border-gray-300 dark:border-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 text-sm font-medium transition-colors"
        >
          <RefreshCw className="w-4 h-4" />
          Refresh
        </button>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-5 shadow-sm">
          <div className="flex items-center gap-3 mb-3">
            <div className="p-2 rounded-xl bg-blue-50 dark:bg-blue-900/30">
              <Hash className="w-4 h-4 text-blue-600 dark:text-blue-400" />
            </div>
            <span className="text-sm text-gray-500 dark:text-gray-400">
              Cache Hits
            </span>
          </div>
          <p className="text-2xl font-bold text-gray-900 dark:text-white">
            {metrics.cache_hits.toLocaleString()}
          </p>
        </div>

        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-5 shadow-sm">
          <div className="flex items-center gap-3 mb-3">
            <div className="p-2 rounded-xl bg-orange-50 dark:bg-orange-900/30">
              <Hash className="w-4 h-4 text-orange-600 dark:text-orange-400" />
            </div>
            <span className="text-sm text-gray-500 dark:text-gray-400">
              Cache Misses
            </span>
          </div>
          <p className="text-2xl font-bold text-gray-900 dark:text-white">
            {metrics.cache_misses.toLocaleString()}
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
            {hitRate}%
          </p>
        </div>

        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-5 shadow-sm">
          <div className="flex items-center gap-3 mb-3">
            <div className="p-2 rounded-xl bg-purple-50 dark:bg-purple-900/30">
              <Zap className="w-4 h-4 text-purple-600 dark:text-purple-400" />
            </div>
            <span className="text-sm text-gray-500 dark:text-gray-400">
              Total Tokens Saved
            </span>
          </div>
          <p className="text-2xl font-bold text-gray-900 dark:text-white">
            {metrics.total_tokens > 0
              ? metrics.total_tokens.toLocaleString()
              : "0"}
          </p>
        </div>
      </div>

      {/* Cache Configuration */}
      {config && (
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 shadow-sm overflow-hidden">
          <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-800 flex items-center gap-2">
            <Database className="w-5 h-5 text-gray-500" />
            <h3 className="text-base font-semibold text-gray-900 dark:text-white">
              Cache Configuration
            </h3>
          </div>
          <div className="p-6">
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-6">
              <div>
                <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                  TTL (Time to Live)
                </p>
                <p className="text-lg font-semibold text-gray-900 dark:text-white">
                  {config.cache_ttl_seconds}s
                </p>
                <p className="text-xs text-gray-400 mt-0.5">
                  ({Math.round(config.cache_ttl_seconds / 60)} minutes)
                </p>
              </div>
              <div>
                <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                  Max Entries
                </p>
                <p className="text-lg font-semibold text-gray-900 dark:text-white">
                  {config.max_cache_entries.toLocaleString()}
                </p>
              </div>
              <div>
                <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                  Database Connected
                </p>
                <p className="text-lg font-semibold text-gray-900 dark:text-white">
                  {config.has_database ? "Yes" : "No (Standalone)"}
                </p>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Info Note */}
      <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-2xl p-4">
        <p className="text-sm text-blue-800 dark:text-blue-300">
          Cache stats are tracked in-memory since the gateway started. Cache requires Redis
          to be configured for persistent storage. In standalone mode, cache counters reset on restart.
        </p>
      </div>
    </div>
  )
}
