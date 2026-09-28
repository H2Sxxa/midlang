import { StrictMode } from "react"
import { createRoot } from "react-dom/client"
import { RouterProvider, createRouter } from "@tanstack/react-router"
import { AuthProvider } from "@/components/auth-provider"
import { useAuth } from "@/hooks/use-auth"
import { routeTree } from "./routeTree.gen"
import "./global.css"

const router = createRouter({
  routeTree,
  context: { auth: undefined },
  defaultPreload: "intent",
  scrollRestoration: true,
})

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
    <AuthProvider>
      <App />
    </AuthProvider>
  </StrictMode>
)