import { fileURLToPath } from "node:url"
import react from "@vitejs/plugin-react"
import tailwindcss from "@tailwindcss/vite"
import { tanstackRouter } from "@tanstack/router-plugin/vite"
import { defineConfig } from "vite"

// https://vite.dev/config/
export default defineConfig({
  // GitHub Pages serves the console from `/<repo>/webui/`; "/" keeps local dev
  // and preview hosts working unchanged. `import.meta.env.BASE_URL` is derived
  // from this value, and the router uses it as its basepath.
  base: process.env.WEBUI_BASE ?? "/",
  plugins: [
    // Must run before react() so route generation and code splitting see the route files.
    tanstackRouter({
      target: "react",
      routesDirectory: "./src/routes",
      generatedRouteTree: "./src/routeTree.gen.ts",
      autoCodeSplitting: true,
      quoteStyle: "double",
    }),
    react(),
    tailwindcss(),
  ],
  resolve: {
    alias: {
      "@/api": fileURLToPath(new URL("./src/api", import.meta.url)),
      "@": fileURLToPath(new URL("./", import.meta.url)),
    },
  },
})
