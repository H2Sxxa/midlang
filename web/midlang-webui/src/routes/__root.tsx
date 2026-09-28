import { Outlet, createRootRouteWithContext } from "@tanstack/react-router"
import { TooltipProvider } from "@/components/ui/tooltip"
import type { AuthContextValue } from "@/hooks/use-auth"

export const Route = createRootRouteWithContext<{
  auth: AuthContextValue | undefined
}>()({
  component: RootComponent,
})

function RootComponent() {
  return (
    <TooltipProvider>
      <Outlet />
    </TooltipProvider>
  )
}