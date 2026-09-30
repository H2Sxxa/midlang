import { useEffect, type ReactNode } from "react"
import { useTheme } from "@/hooks/use-theme"

type ThemeProviderProps = {
  readonly children: ReactNode
}

/**
 * Mirrors the resolved theme onto the root element so Tailwind's `dark` variant
 * and the native color scheme stay in sync with the persisted preference.
 */
export function ThemeProvider({ children }: ThemeProviderProps) {
  const { resolvedTheme } = useTheme()

  useEffect(() => {
    document.documentElement.classList.toggle("dark", resolvedTheme === "dark")
  }, [resolvedTheme])

  return <>{children}</>
}
