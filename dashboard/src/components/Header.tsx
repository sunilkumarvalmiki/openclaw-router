"use client"

import { useSession, signOut } from "next-auth/react"
import { usePathname } from "next/navigation"
import { LogOut, User } from "lucide-react"
import { ThemeToggle } from "./ThemeToggle"

const breadcrumbMap: Record<string, string> = {
  "/dashboard": "Overview",
  "/dashboard/analytics": "Analytics",
  "/dashboard/costs": "Costs",
  "/dashboard/providers": "Providers",
  "/dashboard/cache": "Cache",
  "/dashboard/settings": "Settings",
}

export function Header() {
  const { data: session } = useSession()
  const pathname = usePathname()

  const pageTitle = breadcrumbMap[pathname] || "Dashboard"

  return (
    <header className="sticky top-0 z-30 bg-white/80 dark:bg-gray-950/80 backdrop-blur-md border-b border-gray-200 dark:border-gray-800">
      <div className="flex items-center justify-between h-16 px-4 lg:px-8">
        {/* Breadcrumb */}
        <div className="flex items-center gap-2 ml-12 lg:ml-0">
          <span className="text-sm text-gray-400 dark:text-gray-500">
            Dashboard
          </span>
          <span className="text-sm text-gray-300 dark:text-gray-600">/</span>
          <span className="text-sm font-medium text-gray-900 dark:text-white">
            {pageTitle}
          </span>
        </div>

        {/* Right side */}
        <div className="flex items-center gap-3">
          <ThemeToggle />

          {/* User info */}
          {session?.user && (
            <div className="flex items-center gap-3">
              <div className="hidden sm:flex items-center gap-2">
                {session.user.image ? (
                  <img
                    src={session.user.image}
                    alt="Avatar"
                    className="w-8 h-8 rounded-full"
                  />
                ) : (
                  <div className="w-8 h-8 rounded-full bg-blue-100 dark:bg-blue-900 flex items-center justify-center">
                    <User className="w-4 h-4 text-blue-600 dark:text-blue-400" />
                  </div>
                )}
                <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                  {session.user.name || session.user.email}
                </span>
              </div>

              <button
                onClick={() => signOut({ callbackUrl: "/login" })}
                className="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
                aria-label="Sign out"
                title="Sign out"
              >
                <LogOut className="w-5 h-5 text-gray-500 dark:text-gray-400" />
              </button>
            </div>
          )}
        </div>
      </div>
    </header>
  )
}
