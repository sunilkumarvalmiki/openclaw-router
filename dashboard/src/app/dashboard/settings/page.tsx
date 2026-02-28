"use client"

import { useEffect, useState, useCallback } from "react"
import {
  RefreshCw,
  Database,
  Palette,
  Sliders,
  Server,
  Clock,
} from "lucide-react"
import { useTheme } from "next-themes"
import {
  fetchConfig,
  fetchMetrics,
  formatUptime,
  type GatewayConfig,
  type DashboardMetrics,
} from "@/lib/api"

export default function SettingsPage() {
  const { theme, setTheme } = useTheme()
  const [config, setConfig] = useState<GatewayConfig | null>(null)
  const [metrics, setMetrics] = useState<DashboardMetrics | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)

  const loadData = useCallback(async () => {
    try {
      setError(null)
      const [c, m] = await Promise.all([fetchConfig(), fetchMetrics()])
      setConfig(c)
      setMetrics(m)
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
  }, [loadData])

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="flex items-center gap-3 text-gray-400">
          <RefreshCw className="w-5 h-5 animate-spin" />
          <span>Loading configuration...</span>
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

  return (
    <div className="space-y-6 max-w-4xl">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
            Settings
          </h2>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Gateway configuration and preferences (read-only from gateway)
          </p>
        </div>
        <button
          onClick={loadData}
          className="p-2 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
          title="Refresh config"
        >
          <RefreshCw className="w-5 h-5" />
        </button>
      </div>

      {/* Gateway Status */}
      {metrics && (
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 shadow-sm overflow-hidden">
          <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-800 flex items-center gap-2">
            <Clock className="w-5 h-5 text-gray-500" />
            <h3 className="text-base font-semibold text-gray-900 dark:text-white">
              Gateway Status
            </h3>
          </div>
          <div className="p-6">
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-6">
              <div>
                <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                  Uptime
                </p>
                <p className="text-lg font-semibold text-gray-900 dark:text-white">
                  {formatUptime(metrics.uptime_seconds)}
                </p>
              </div>
              <div>
                <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                  Total Requests Handled
                </p>
                <p className="text-lg font-semibold text-gray-900 dark:text-white">
                  {metrics.total_requests.toLocaleString()}
                </p>
              </div>
              <div>
                <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                  Success Rate
                </p>
                <p className="text-lg font-semibold text-green-600 dark:text-green-400">
                  {metrics.total_requests > 0
                    ? `${(
                        (metrics.successful_requests /
                          metrics.total_requests) *
                        100
                      ).toFixed(1)}%`
                    : "N/A"}
                </p>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Gateway Configuration */}
      {config && (
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 shadow-sm overflow-hidden">
          <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-800 flex items-center gap-2">
            <Sliders className="w-5 h-5 text-gray-500" />
            <h3 className="text-base font-semibold text-gray-900 dark:text-white">
              Gateway Configuration
            </h3>
          </div>
          <div className="p-6">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
              <div>
                <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                  Host
                </p>
                <p className="text-sm font-semibold text-gray-900 dark:text-white font-mono">
                  {config.host}:{config.port}
                </p>
              </div>
              <div>
                <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                  Default Cost Profile
                </p>
                <p className="text-sm font-semibold text-gray-900 dark:text-white uppercase">
                  {config.default_cost_profile}
                </p>
              </div>
              <div>
                <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                  Database
                </p>
                <p className="text-sm font-semibold text-gray-900 dark:text-white">
                  {config.has_database ? "Connected" : "Standalone (no DB)"}
                </p>
              </div>
              <div>
                <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                  Dashboard URL
                </p>
                <p className="text-sm font-semibold text-gray-900 dark:text-white font-mono">
                  {config.dashboard_url}
                </p>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Cache Settings */}
      {config && (
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 shadow-sm overflow-hidden">
          <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-800 flex items-center gap-2">
            <Database className="w-5 h-5 text-gray-500" />
            <h3 className="text-base font-semibold text-gray-900 dark:text-white">
              Cache Settings
            </h3>
          </div>
          <div className="p-6">
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-6">
              <div>
                <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                  TTL (seconds)
                </p>
                <p className="text-lg font-semibold text-gray-900 dark:text-white">
                  {config.cache_ttl_seconds.toLocaleString()}
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
                  Pricing Sync Interval
                </p>
                <p className="text-lg font-semibold text-gray-900 dark:text-white">
                  {config.pricing_sync_interval_seconds}s
                </p>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Active Providers */}
      {config && (
        <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 shadow-sm overflow-hidden">
          <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-800 flex items-center gap-2">
            <Server className="w-5 h-5 text-gray-500" />
            <h3 className="text-base font-semibold text-gray-900 dark:text-white">
              Active Providers
            </h3>
          </div>
          <div className="p-6">
            <div className="flex flex-wrap gap-2">
              {config.providers.map((p) => (
                <span
                  key={p}
                  className="inline-flex items-center px-3 py-1.5 rounded-full text-sm font-medium bg-blue-50 dark:bg-blue-900/30 text-blue-700 dark:text-blue-400 capitalize"
                >
                  {p}
                </span>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* Appearance */}
      <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-800 flex items-center gap-2">
          <Palette className="w-5 h-5 text-gray-500" />
          <h3 className="text-base font-semibold text-gray-900 dark:text-white">
            Appearance
          </h3>
        </div>
        <div className="p-6">
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">
            Theme
          </label>
          <div className="flex gap-3">
            {(["system", "light", "dark"] as const).map((t) => (
              <button
                key={t}
                onClick={() => setTheme(t)}
                className={`px-4 py-2.5 rounded-xl text-sm font-medium border transition-colors ${
                  theme === t
                    ? "bg-blue-50 dark:bg-blue-950 border-blue-300 dark:border-blue-700 text-blue-600 dark:text-blue-400"
                    : "bg-white dark:bg-gray-800 border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-750"
                }`}
              >
                {t.charAt(0).toUpperCase() + t.slice(1)}
              </button>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}
