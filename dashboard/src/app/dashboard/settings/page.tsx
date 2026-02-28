"use client"

import { useState } from "react"
import {
  Save,
  Eye,
  EyeOff,
  Trash2,
  AlertTriangle,
  Database,
  Palette,
  Shield,
  Gauge,
  Sliders,
} from "lucide-react"
import { useTheme } from "next-themes"

interface ProviderKey {
  name: string
  key: string
  visible: boolean
}

export default function SettingsPage() {
  const { theme, setTheme } = useTheme()

  // General
  const [costProfile, setCostProfile] = useState("AUTO")

  // API Keys
  const [providerKeys, setProviderKeys] = useState<ProviderKey[]>([
    { name: "Groq", key: "gsk_****************************", visible: false },
    { name: "OpenAI", key: "sk-****************************", visible: false },
    {
      name: "Anthropic",
      key: "sk-ant-****************************",
      visible: false,
    },
    { name: "Google", key: "AIza****************************", visible: false },
    { name: "GitHub", key: "ghp_****************************", visible: false },
  ])

  // Rate Limits
  const [rateLimits, setRateLimits] = useState({
    rpm: "60",
    tpm: "100000",
  })

  // Cache
  const [cacheSettings, setCacheSettings] = useState({
    ttl: "3600",
    maxEntries: "10000",
  })

  // Danger Zone
  const [showResetConfirm, setShowResetConfirm] = useState(false)
  const [saveStatus, setSaveStatus] = useState<string | null>(null)

  const toggleKeyVisibility = (index: number) => {
    setProviderKeys((prev) =>
      prev.map((p, i) =>
        i === index ? { ...p, visible: !p.visible } : p
      )
    )
  }

  const updateKey = (index: number, key: string) => {
    setProviderKeys((prev) =>
      prev.map((p, i) => (i === index ? { ...p, key } : p))
    )
  }

  const handleSave = () => {
    setSaveStatus("Saving...")
    setTimeout(() => {
      setSaveStatus("Settings saved successfully!")
      setTimeout(() => setSaveStatus(null), 3000)
    }, 800)
  }

  const handleReset = () => {
    setCostProfile("AUTO")
    setRateLimits({ rpm: "60", tpm: "100000" })
    setCacheSettings({ ttl: "3600", maxEntries: "10000" })
    setShowResetConfirm(false)
    setSaveStatus("All settings reset to defaults")
    setTimeout(() => setSaveStatus(null), 3000)
  }

  const handleClearCache = () => {
    setSaveStatus("Cache cleared successfully!")
    setTimeout(() => setSaveStatus(null), 3000)
  }

  return (
    <div className="space-y-6 max-w-4xl">
      <div>
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
          Settings
        </h2>
        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
          Configure your OpenClaw Router preferences
        </p>
      </div>

      {/* Save Status Toast */}
      {saveStatus && (
        <div className="fixed top-4 right-4 z-50 px-4 py-3 rounded-xl bg-green-500 text-white text-sm font-medium shadow-lg animate-pulse">
          {saveStatus}
        </div>
      )}

      {/* General Section */}
      <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-800 flex items-center gap-2">
          <Sliders className="w-5 h-5 text-gray-500" />
          <h3 className="text-base font-semibold text-gray-900 dark:text-white">
            General
          </h3>
        </div>
        <div className="p-6">
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5">
              Default Cost Profile
            </label>
            <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
              Controls how the router selects providers based on cost
            </p>
            <select
              value={costProfile}
              onChange={(e) => setCostProfile(e.target.value)}
              className="w-full sm:w-64 px-4 py-2.5 rounded-xl border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            >
              <option value="FREE">FREE - Only free providers</option>
              <option value="ECO">ECO - Cheapest available</option>
              <option value="AUTO">AUTO - Balanced cost/quality</option>
              <option value="PREMIUM">
                PREMIUM - Best quality, any cost
              </option>
            </select>
          </div>
        </div>
      </div>

      {/* Provider API Keys */}
      <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-800 flex items-center gap-2">
          <Shield className="w-5 h-5 text-gray-500" />
          <h3 className="text-base font-semibold text-gray-900 dark:text-white">
            Provider API Keys
          </h3>
        </div>
        <div className="p-6 space-y-4">
          {providerKeys.map((provider, index) => (
            <div key={provider.name}>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5">
                {provider.name} API Key
              </label>
              <div className="relative">
                <input
                  type={provider.visible ? "text" : "password"}
                  value={provider.key}
                  onChange={(e) => updateKey(index, e.target.value)}
                  className="w-full pr-10 px-4 py-2.5 rounded-xl border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-900 dark:text-white font-mono text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                />
                <button
                  onClick={() => toggleKeyVisibility(index)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                  aria-label={
                    provider.visible ? "Hide key" : "Show key"
                  }
                >
                  {provider.visible ? (
                    <EyeOff className="w-4 h-4" />
                  ) : (
                    <Eye className="w-4 h-4" />
                  )}
                </button>
              </div>
            </div>
          ))}
          <button
            onClick={handleSave}
            className="inline-flex items-center gap-2 px-5 py-2.5 rounded-xl bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium transition-colors"
          >
            <Save className="w-4 h-4" />
            Save API Keys
          </button>
        </div>
      </div>

      {/* Rate Limits */}
      <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-800 flex items-center gap-2">
          <Gauge className="w-5 h-5 text-gray-500" />
          <h3 className="text-base font-semibold text-gray-900 dark:text-white">
            Rate Limits
          </h3>
        </div>
        <div className="p-6">
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5">
                Requests per Minute (RPM)
              </label>
              <input
                type="number"
                value={rateLimits.rpm}
                onChange={(e) =>
                  setRateLimits((prev) => ({
                    ...prev,
                    rpm: e.target.value,
                  }))
                }
                className="w-full px-4 py-2.5 rounded-xl border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5">
                Tokens per Minute (TPM)
              </label>
              <input
                type="number"
                value={rateLimits.tpm}
                onChange={(e) =>
                  setRateLimits((prev) => ({
                    ...prev,
                    tpm: e.target.value,
                  }))
                }
                className="w-full px-4 py-2.5 rounded-xl border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              />
            </div>
          </div>
          <button
            onClick={handleSave}
            className="inline-flex items-center gap-2 px-5 py-2.5 rounded-xl bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium transition-colors mt-4"
          >
            <Save className="w-4 h-4" />
            Save Rate Limits
          </button>
        </div>
      </div>

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

      {/* Cache Section */}
      <div className="bg-white dark:bg-gray-900 rounded-2xl border border-gray-200 dark:border-gray-800 shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-800 flex items-center gap-2">
          <Database className="w-5 h-5 text-gray-500" />
          <h3 className="text-base font-semibold text-gray-900 dark:text-white">
            Cache
          </h3>
        </div>
        <div className="p-6">
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5">
                TTL (seconds)
              </label>
              <input
                type="number"
                value={cacheSettings.ttl}
                onChange={(e) =>
                  setCacheSettings((prev) => ({
                    ...prev,
                    ttl: e.target.value,
                  }))
                }
                className="w-full px-4 py-2.5 rounded-xl border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5">
                Max Entries
              </label>
              <input
                type="number"
                value={cacheSettings.maxEntries}
                onChange={(e) =>
                  setCacheSettings((prev) => ({
                    ...prev,
                    maxEntries: e.target.value,
                  }))
                }
                className="w-full px-4 py-2.5 rounded-xl border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              />
            </div>
          </div>
          <div className="flex gap-3 mt-4">
            <button
              onClick={handleSave}
              className="inline-flex items-center gap-2 px-5 py-2.5 rounded-xl bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium transition-colors"
            >
              <Save className="w-4 h-4" />
              Save Cache Settings
            </button>
            <button
              onClick={handleClearCache}
              className="inline-flex items-center gap-2 px-5 py-2.5 rounded-xl border border-gray-300 dark:border-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 text-sm font-medium transition-colors"
            >
              <Trash2 className="w-4 h-4" />
              Clear Cache
            </button>
          </div>
        </div>
      </div>

      {/* Danger Zone */}
      <div className="bg-white dark:bg-gray-900 rounded-2xl border border-red-200 dark:border-red-900/50 shadow-sm overflow-hidden">
        <div className="px-6 py-4 border-b border-red-200 dark:border-red-900/50 flex items-center gap-2">
          <AlertTriangle className="w-5 h-5 text-red-500" />
          <h3 className="text-base font-semibold text-red-600 dark:text-red-400">
            Danger Zone
          </h3>
        </div>
        <div className="p-6">
          <p className="text-sm text-gray-600 dark:text-gray-400 mb-4">
            This action will reset all settings to their default values. This
            cannot be undone.
          </p>
          {!showResetConfirm ? (
            <button
              onClick={() => setShowResetConfirm(true)}
              className="inline-flex items-center gap-2 px-5 py-2.5 rounded-xl bg-red-600 hover:bg-red-700 text-white text-sm font-medium transition-colors"
            >
              <Trash2 className="w-4 h-4" />
              Reset All Settings
            </button>
          ) : (
            <div className="p-4 rounded-xl bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800">
              <p className="text-sm font-medium text-red-800 dark:text-red-300 mb-3">
                Are you sure? This will reset all settings to defaults.
              </p>
              <div className="flex gap-3">
                <button
                  onClick={handleReset}
                  className="px-4 py-2 rounded-lg bg-red-600 hover:bg-red-700 text-white text-sm font-medium transition-colors"
                >
                  Yes, Reset Everything
                </button>
                <button
                  onClick={() => setShowResetConfirm(false)}
                  className="px-4 py-2 rounded-lg border border-gray-300 dark:border-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 text-sm font-medium transition-colors"
                >
                  Cancel
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
