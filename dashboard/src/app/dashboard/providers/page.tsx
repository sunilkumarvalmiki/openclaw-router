"use client"

import { useEffect, useState, useCallback } from "react"
import {
  Server,
  ChevronDown,
  ChevronRight,
  Clock,
  AlertTriangle,
  Activity,
  RefreshCw,
} from "lucide-react"
import {
  fetchProviderStats,
  fetchModels,
  type ProviderMetrics,
  type ModelInfo,
} from "@/lib/api"

function ProviderCard({
  provider,
  models,
}: {
  provider: ProviderMetrics
  models: ModelInfo[]
}) {
  const [expanded, setExpanded] = useState(false)

  const providerModels = models.filter((m) => m.provider === provider.name)

  return (
    <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 shadow-sm overflow-hidden">
      <div
        className="p-6 cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
        onClick={() => setExpanded(!expanded)}
      >
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-3">
            <div className="p-2.5 rounded-xl bg-gray-100 dark:bg-gray-800">
              <Server className="w-5 h-5 text-gray-600 dark:text-gray-400" />
            </div>
            <div>
              <h3 className="text-base font-semibold text-gray-900 dark:text-white capitalize">
                {provider.name}
              </h3>
              <div className="flex items-center gap-2 mt-1">
                <span
                  className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium ${
                    provider.is_healthy
                      ? "bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400"
                      : "bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-400"
                  }`}
                >
                  <span
                    className={`w-1.5 h-1.5 rounded-full ${
                      provider.is_healthy ? "bg-green-500" : "bg-red-500"
                    }`}
                  />
                  {provider.is_healthy ? "Healthy" : "Unhealthy"}
                </span>
              </div>
            </div>
          </div>
          <div className="flex items-center gap-2 text-gray-400">
            <span className="text-xs text-gray-400 dark:text-gray-500">
              {providerModels.length} models
            </span>
            {expanded ? (
              <ChevronDown className="w-5 h-5" />
            ) : (
              <ChevronRight className="w-5 h-5" />
            )}
          </div>
        </div>

        {/* Quick Stats */}
        <div className="grid grid-cols-3 gap-4 mt-4">
          <div className="flex items-center gap-2">
            <Clock className="w-4 h-4 text-gray-400" />
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400">
                Avg Latency
              </p>
              <p className="text-sm font-semibold text-gray-900 dark:text-white">
                {provider.avg_latency_ms > 0
                  ? `${Math.round(provider.avg_latency_ms)}ms`
                  : "N/A"}
              </p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <AlertTriangle className="w-4 h-4 text-gray-400" />
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400">
                Error Rate
              </p>
              <p className="text-sm font-semibold text-gray-900 dark:text-white">
                {provider.error_rate.toFixed(1)}%
              </p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Activity className="w-4 h-4 text-gray-400" />
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400">
                Total Requests
              </p>
              <p className="text-sm font-semibold text-gray-900 dark:text-white">
                {provider.total_requests.toLocaleString()}
              </p>
            </div>
          </div>
        </div>

        {/* Tokens Used */}
        <div className="mt-3 text-xs text-gray-500 dark:text-gray-400">
          Tokens processed: {provider.total_tokens.toLocaleString()}
        </div>
      </div>

      {/* Expanded: Model List */}
      {expanded && providerModels.length > 0 && (
        <div className="border-t border-gray-200 dark:border-gray-800 px-6 py-4 bg-gray-50 dark:bg-gray-800/30">
          <h4 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">
            Available Models
          </h4>
          <div className="space-y-2">
            {providerModels.map((model) => (
              <div
                key={model.id}
                className="flex items-center justify-between p-3 rounded-xl bg-white dark:bg-gray-900 border border-gray-100 dark:border-gray-800"
              >
                <span className="text-sm font-medium text-gray-900 dark:text-white">
                  {model.id}
                </span>
                <span
                  className={`px-1.5 py-0.5 rounded text-xs font-medium ${
                    model.is_available
                      ? "bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400"
                      : "bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400"
                  }`}
                >
                  {model.is_available ? "Available" : "Unavailable"}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

export default function ProvidersPage() {
  const [providers, setProviders] = useState<ProviderMetrics[]>([])
  const [models, setModels] = useState<ModelInfo[]>([])
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)

  const loadData = useCallback(async () => {
    try {
      setError(null)
      const [p, m] = await Promise.all([fetchProviderStats(), fetchModels()])
      setProviders(p)
      setModels(m)
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
          <span>Loading providers...</span>
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

  const healthyCount = providers.filter((p) => p.is_healthy).length
  const unhealthyCount = providers.filter((p) => !p.is_healthy).length

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
            Provider Health
          </h2>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Real-time provider status and performance metrics
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

      {/* Summary */}
      <div className="flex items-center gap-4">
        <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-sm font-medium bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400">
          <span className="w-2 h-2 rounded-full bg-green-500" />
          {healthyCount} Healthy
        </span>
        {unhealthyCount > 0 && (
          <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-sm font-medium bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-400">
            <span className="w-2 h-2 rounded-full bg-red-500" />
            {unhealthyCount} Unhealthy
          </span>
        )}
      </div>

      {/* Provider Cards */}
      <div className="space-y-4">
        {providers.map((provider) => (
          <ProviderCard
            key={provider.name}
            provider={provider}
            models={models}
          />
        ))}
        {providers.length === 0 && (
          <div className="text-center text-gray-400 py-12">
            No providers configured.
          </div>
        )}
      </div>
    </div>
  )
}
