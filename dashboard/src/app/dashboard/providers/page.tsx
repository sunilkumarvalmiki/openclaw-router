"use client"

import { useEffect, useState } from "react"
import {
  Server,
  ChevronDown,
  ChevronRight,
  Shield,
  ShieldAlert,
  ShieldOff,
  Clock,
  AlertTriangle,
  Activity,
} from "lucide-react"
import { fetchProviders } from "@/lib/api"

interface Provider {
  name: string
  status: "healthy" | "degraded" | "down"
  circuitBreaker: "closed" | "open" | "half_open"
  rateLimit: { used: number; total: number; unit: string }
  models: string[]
  avgLatency: number
  errorRate: number
  totalRequests: number
}

const statusConfig = {
  healthy: {
    label: "Healthy",
    bgColor: "bg-green-100 dark:bg-green-900/30",
    textColor: "text-green-700 dark:text-green-400",
    dotColor: "bg-green-500",
  },
  degraded: {
    label: "Degraded",
    bgColor: "bg-yellow-100 dark:bg-yellow-900/30",
    textColor: "text-yellow-700 dark:text-yellow-400",
    dotColor: "bg-yellow-500",
  },
  down: {
    label: "Down",
    bgColor: "bg-red-100 dark:bg-red-900/30",
    textColor: "text-red-700 dark:text-red-400",
    dotColor: "bg-red-500",
  },
}

const circuitBreakerConfig = {
  closed: {
    label: "Closed",
    icon: Shield,
    color: "text-green-600 dark:text-green-400",
  },
  half_open: {
    label: "Half-Open",
    icon: ShieldAlert,
    color: "text-yellow-600 dark:text-yellow-400",
  },
  open: {
    label: "Open",
    icon: ShieldOff,
    color: "text-red-600 dark:text-red-400",
  },
}

function ProviderCard({ provider }: { provider: Provider }) {
  const [expanded, setExpanded] = useState(false)
  const status = statusConfig[provider.status]
  const cb = circuitBreakerConfig[provider.circuitBreaker]
  const CbIcon = cb.icon
  const rateLimitPercent =
    provider.rateLimit.total > 0
      ? (provider.rateLimit.used / provider.rateLimit.total) * 100
      : 0

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
              <h3 className="text-base font-semibold text-gray-900 dark:text-white">
                {provider.name}
              </h3>
              <div className="flex items-center gap-2 mt-1">
                <span
                  className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium ${status.bgColor} ${status.textColor}`}
                >
                  <span
                    className={`w-1.5 h-1.5 rounded-full ${status.dotColor}`}
                  />
                  {status.label}
                </span>
                <span className={`inline-flex items-center gap-1 text-xs ${cb.color}`}>
                  <CbIcon className="w-3.5 h-3.5" />
                  {cb.label}
                </span>
              </div>
            </div>
          </div>
          <div className="flex items-center gap-2 text-gray-400">
            <span className="text-xs text-gray-400 dark:text-gray-500">
              {provider.models.length} models
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
                {provider.avgLatency > 0 ? `${provider.avgLatency}ms` : "N/A"}
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
                {provider.errorRate}%
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
                {provider.totalRequests.toLocaleString()}
              </p>
            </div>
          </div>
        </div>

        {/* Rate Limit Bar */}
        <div className="mt-4">
          <div className="flex items-center justify-between text-xs mb-1">
            <span className="text-gray-500 dark:text-gray-400">
              Rate Limit ({provider.rateLimit.unit})
            </span>
            <span className="text-gray-600 dark:text-gray-300 font-medium">
              {provider.rateLimit.used} / {provider.rateLimit.total}
            </span>
          </div>
          <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
            <div
              className={`h-2 rounded-full transition-all ${
                rateLimitPercent > 90
                  ? "bg-red-500"
                  : rateLimitPercent > 70
                  ? "bg-yellow-500"
                  : "bg-green-500"
              }`}
              style={{ width: `${Math.min(rateLimitPercent, 100)}%` }}
            />
          </div>
        </div>
      </div>

      {/* Expanded: Model List */}
      {expanded && (
        <div className="border-t border-gray-200 dark:border-gray-800 px-6 py-4 bg-gray-50 dark:bg-gray-800/30">
          <h4 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">
            Available Models
          </h4>
          <div className="space-y-2">
            {provider.models.map((model) => (
              <div
                key={model}
                className="flex items-center justify-between p-3 rounded-xl bg-white dark:bg-gray-900 border border-gray-100 dark:border-gray-800"
              >
                <span className="text-sm font-medium text-gray-900 dark:text-white">
                  {model}
                </span>
                <div className="flex items-center gap-4 text-xs text-gray-500 dark:text-gray-400">
                  <span>
                    {Math.floor(Math.random() * 1000) + 100} req/hr
                  </span>
                  <span>{Math.floor(Math.random() * 300) + 50}ms avg</span>
                  <span
                    className={`px-1.5 py-0.5 rounded text-xs font-medium ${
                      provider.status === "down"
                        ? "bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400"
                        : "bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400"
                    }`}
                  >
                    {provider.status === "down" ? "Offline" : "Online"}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

export default function ProvidersPage() {
  const [providers, setProviders] = useState<Provider[]>([])

  useEffect(() => {
    async function loadData() {
      const data = await fetchProviders()
      setProviders(data)
    }
    loadData()
  }, [])

  const healthyCount = providers.filter((p) => p.status === "healthy").length
  const degradedCount = providers.filter((p) => p.status === "degraded").length
  const downCount = providers.filter((p) => p.status === "down").length

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
          Provider Health
        </h2>
        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
          Monitor provider status, circuit breakers, and rate limits
        </p>
      </div>

      {/* Summary */}
      <div className="flex items-center gap-4">
        <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-sm font-medium bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400">
          <span className="w-2 h-2 rounded-full bg-green-500" />
          {healthyCount} Healthy
        </span>
        {degradedCount > 0 && (
          <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-sm font-medium bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-400">
            <span className="w-2 h-2 rounded-full bg-yellow-500" />
            {degradedCount} Degraded
          </span>
        )}
        {downCount > 0 && (
          <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-sm font-medium bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-400">
            <span className="w-2 h-2 rounded-full bg-red-500" />
            {downCount} Down
          </span>
        )}
      </div>

      {/* Provider Cards */}
      <div className="space-y-4">
        {providers.map((provider) => (
          <ProviderCard key={provider.name} provider={provider} />
        ))}
      </div>
    </div>
  )
}
