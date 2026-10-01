import { Link, Outlet, createRootRouteWithContext } from "@tanstack/react-router"
import { ThemeProvider } from "@/components/theme-provider"
import { Toaster } from "@/components/ui/sonner"
import { TooltipProvider } from "@/components/ui/tooltip"
import type { AuthContextValue } from "@/hooks/use-auth"

export const Route = createRootRouteWithContext<{
  auth: AuthContextValue | undefined
}>()({
  notFoundComponent: NotFoundPage,
  component: RootComponent,
})

function NotFoundPage() {
  return (
    <main className="flex min-h-dvh flex-col items-center justify-center gap-3 p-6 text-center">
      <h1 className="text-2xl font-semibold tracking-tight">Page not found</h1>
      <p className="text-sm text-muted-foreground">The page you requested does not exist.</p>
      <Link className="text-sm text-primary underline underline-offset-4" to="/">
        Return to overview
      </Link>
    </main>
  )
}

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
