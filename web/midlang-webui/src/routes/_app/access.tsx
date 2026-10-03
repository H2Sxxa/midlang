import { KeyIcon, ShieldCheckIcon } from "@phosphor-icons/react"
import { Link, Outlet, createFileRoute, useMatchRoute } from "@tanstack/react-router"
import { cn } from "cn"
import { useAuth } from "@/hooks/use-auth"

export const Route = createFileRoute("/_app/access")({
  component: AccessLayout,
})

const accessTabs = [
  { to: "/access/tokens", label: "API Tokens", icon: KeyIcon, permission: "token:read" },
  {
    to: "/access/groups",
    label: "Permission Groups",
    icon: ShieldCheckIcon,
    permission: "permission:read",
  },
] as const

function AccessLayout() {
  const auth = useAuth()
  const matchRoute = useMatchRoute()
  const visibleTabs = accessTabs.filter((tab) => auth.can(tab.permission))

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Access</h1>
        <p className="mt-1 text-muted-foreground">
          Manage the API tokens that reach this server and the permission groups they inherit.
        </p>
      </div>
      <div className="flex items-center gap-1 border-b">
        {visibleTabs.map((tab) => (
          <Link
            key={tab.to}
            to={tab.to}
            className={cn(
              "inline-flex items-center gap-1.5 border-b-2 border-transparent px-3 py-2 text-xs font-medium text-muted-foreground transition-colors hover:text-foreground",
              matchRoute({ to: tab.to }) && "border-foreground text-foreground",
            )}
          >
            <tab.icon />
            {tab.label}
          </Link>
        ))}
      </div>
      <Outlet />
    </div>
  )
}
