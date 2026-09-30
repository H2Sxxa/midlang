import { Outlet, createRootRouteWithContext } from "@tanstack/react-router"
import { ThemeProvider } from "@/components/theme-provider"
import { Toaster } from "@/components/ui/sonner"
import { TooltipProvider } from "@/components/ui/tooltip"
import type { AuthContextValue } from "@/hooks/use-auth"

export const Route = createRootRouteWithContext<{
  auth: AuthContextValue | undefined
}>()({
  component: RootComponent,
})

function RootComponent() {
  return (
    <ThemeProvider>
      <TooltipProvider>
        <Outlet />
      </TooltipProvider>
      <Toaster />
    </ThemeProvider>
  )
}
