import { defineConfig } from "vitepress"

/**
 * VitePress requires `base` to start and end with a slash, so accept either
 * form from the environment (`midlang`, `/midlang`, `/midlang/`).
 */
function normalizeBase(rawBase: string): string {
  const trimmed = rawBase.replace(/^\/+/, "").replace(/\/+$/, "")

  return trimmed === "" ? "/" : `/${trimmed}/`
}

/** GitHub Pages serves the docs from `/<repo>/`; "/" keeps local dev unchanged. */
const docsBase = normalizeBase(process.env.DOCS_BASE ?? "/")

/** Sub-path of the console, e.g. `/<repo>/webui/`. Unset outside Pages builds. */
const webuiBase = process.env.WEBUI_BASE

/**
 * GitHub Pages answers every missing path with the single 404 page at the site
 * root, which is where the console's client-side routes end up. Hand them back
 * to the SPA together with the requested path, which the console replays on
 * boot. Keep the storage key in sync with `web/midlang-webui/src/main.tsx`.
 */
const webuiFallbackScript =
  webuiBase === undefined
    ? ""
    : `
    <script>
      ;(function () {
        var webuiBase = ${JSON.stringify(webuiBase)}
        var path = window.location.pathname
        if (path.indexOf(webuiBase) !== 0 || path === webuiBase) {
          return
        }
        try {
          window.sessionStorage.setItem(
            "midlang:spa-redirect",
            path.slice(webuiBase.length - 1) + window.location.search + window.location.hash
          )
        } catch (error) {
          console.warn("midlang: cannot remember the requested console route", error)
        }
        window.location.replace(webuiBase)
      })()
    </script>
`

export default defineConfig({
  base: docsBase,
  lang: "en",
  title: "MidLang",
  description:
    "Translation middleware that decouples translation resources from business data.",
  lastUpdated: true,
  transformHtml(code, _id, context) {
    if (webuiFallbackScript === "" || context.page !== "404.md") {
      return
    }

    return code.replace("<head>", `<head>${webuiFallbackScript}`)
  },
  themeConfig: {
    nav: [{ text: "Guide", link: "/guide/getting-started" }],
    sidebar: {
      "/guide/": [
        {
          text: "Guide",
          items: [{ text: "Getting started", link: "/guide/getting-started" }],
        },
      ],
    },
    search: {
      provider: "local",
    },
    outline: {
      level: [2, 3],
    },
    footer: {
      message: "Early development: APIs, formats and routes can still change.",
    },
  },
})
