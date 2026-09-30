import { useEffect, useState } from "react"
import { create } from "zustand"
import { persist } from "zustand/middleware"

/**
 * Storage key for the persisted theme preference. The same key is read by the
 * inline boot script in `index.html` that applies the theme before React mounts,
 * so both places must stay in sync.
 */
export const THEME_STORAGE_KEY = "midlang-webui:theme"

const DARK_MODE_QUERY = "(prefers-color-scheme: dark)"

/** Theme requested by the user, where `system` follows the operating system. */
export type ThemePreference = "light" | "dark" | "system"

/** Theme that is actually painted, after resolving `system` against the OS setting. */
export type ResolvedTheme = "light" | "dark"

export type ThemeOption = {
  readonly value: ThemePreference
  readonly label: string
}

export const themeOptions: ReadonlyArray<ThemeOption> = [
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
  { value: "system", label: "System" },
]

type ThemeStore = {
  readonly preference: ThemePreference
  readonly setPreference: (preference: ThemePreference) => void
}

export const useThemeStore = create<ThemeStore>()(
  persist(
    (set) => ({
      preference: "system",
      setPreference: (preference) => {
        set({ preference })
      },
    }),
    {
      name: THEME_STORAGE_KEY,
      version: 1,
    },
  ),
)

export function resolveTheme(
  preference: ThemePreference,
  systemTheme: ResolvedTheme,
): ResolvedTheme {
  return preference === "system" ? systemTheme : preference
}

function readSystemTheme(): ResolvedTheme {
  return window.matchMedia(DARK_MODE_QUERY).matches ? "dark" : "light"
}

/** Tracks the operating system color scheme so `system` preferences stay live. */
export function useSystemTheme(): ResolvedTheme {
  const [systemTheme, setSystemTheme] = useState<ResolvedTheme>(readSystemTheme)

  useEffect(() => {
    const query = window.matchMedia(DARK_MODE_QUERY)
    const handleChange = (event: MediaQueryListEvent): void => {
      setSystemTheme(event.matches ? "dark" : "light")
    }

    query.addEventListener("change", handleChange)

    return () => {
      query.removeEventListener("change", handleChange)
    }
  }, [])

  return systemTheme
}

export type ThemeState = {
  readonly preference: ThemePreference
  readonly resolvedTheme: ResolvedTheme
  readonly setPreference: (preference: ThemePreference) => void
}

/** Reads the persisted theme preference and the theme it currently resolves to. */
export function useTheme(): ThemeState {
  const preference = useThemeStore((state) => state.preference)
  const setPreference = useThemeStore((state) => state.setPreference)
  const systemTheme = useSystemTheme()

  return {
    preference,
    resolvedTheme: resolveTheme(preference, systemTheme),
    setPreference,
  }
}

