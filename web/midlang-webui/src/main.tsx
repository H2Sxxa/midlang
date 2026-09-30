import { StrictMode } from "react"
import { createRoot } from "react-dom/client"
import { RouterProvider, createRouter } from "@tanstack/react-router"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { AuthProvider } from "@/components/auth-provider"
import { useAuth } from "@/hooks/use-auth"
import { routeTree } from "./routeTree.gen"
import "./global.css"

/**
 * Key shared with the docs' 404 page, which bounces unmatched console routes
 * here because GitHub Pages cannot rewrite URLs to a single-page app.
 */
const SPA_REDIRECT_KEY = "midlang:spa-redirect"

/**
 * Replays a route that the 404 page remembered, so deep links and reloads land
 * on the requested screen instead of the console root. Must run before the
 * router reads the URL.
 */
function restoreRedirectedPath(): void {
  const redirectedPath = window.sessionStorage.getItem(SPA_REDIRECT_KEY)

  if (redirectedPath === null) {
    return
  }

  window.sessionStorage.removeItem(SPA_REDIRECT_KEY)
  window.history.replaceState(null, "", `${import.meta.env.BASE_URL.replace(/\/$/, "")}${redirectedPath}`)
}

restoreRedirectedPath()

const router = createRouter({
  routeTree,
  // Same prefix Vite bakes into the assets, so links and redirects stay inside
  // the deployed sub-path.
  basepath: import.meta.env.BASE_URL,
  context: { auth: undefined },
  defaultPreload: "intent",
  scrollRestoration: true,
})

const queryClient = new QueryClient()

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router
  }
}

function App() {
  const auth = useAuth()

  return <RouterProvider router={router} context={{ auth }} />
}

const rootElement = document.getElementById("root")

if (!rootElement) {
  throw new Error("Failed to mount midlang-webui: #root element is missing from index.html")
}

createRoot(rootElement).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <AuthProvider>
        <App />
      </AuthProvider>
    </QueryClientProvider>
  </StrictMode>
)
