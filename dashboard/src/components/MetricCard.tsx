"use client"

import { TrendingUp, TrendingDown } from "lucide-react"
import type { LucideIcon } from "lucide-react"

interface MetricCardProps {
  title: string
  value: string
  trend: number
  trendLabel?: string
  icon: LucideIcon
  iconColor?: string
}

export function MetricCard({
  title,
  value,
  trend,
  trendLabel = "vs last period",
  icon: Icon,
  iconColor = "text-blue-600 dark:text-blue-400",
}: MetricCardProps) {
  const isPositive = trend >= 0

  return (
    <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 p-6 shadow-sm">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <p className="text-sm font-medium text-gray-500 dark:text-gray-400">
            {title}
          </p>
          <p className="text-2xl font-bold text-gray-900 dark:text-white mt-1">
            {value}
          </p>
        </div>
        <div
          className={`p-3 rounded-xl bg-gray-50 dark:bg-gray-800 ${iconColor}`}
        >
          <Icon className="w-5 h-5" />
        </div>
      </div>

      <div className="flex items-center gap-1.5 mt-3">
        {isPositive ? (
          <TrendingUp className="w-4 h-4 text-green-500" />
        ) : (
          <TrendingDown className="w-4 h-4 text-red-500" />
        )}
        <span
          className={`text-sm font-medium ${
            isPositive ? "text-green-500" : "text-red-500"
          }`}
        >
          {isPositive ? "+" : ""}
          {trend}%
        </span>
        <span className="text-xs text-gray-400 dark:text-gray-500">
          {trendLabel}
        </span>
      </div>
    </div>
  )
}
