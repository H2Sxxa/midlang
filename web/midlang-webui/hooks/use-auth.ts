import { createContext, useContext } from "react"

export type AuthStatus =
  | "loading"
  | "unauthenticated"
  | "authenticating"
  | "authenticated"
  | "error"

export type AuthContextValue = {
  readonly token: string | null
  readonly remoteUrl: string
  readonly status: AuthStatus
  readonly login: (token: string, remoteUrl: string) => Promise<void>
  readonly logout: () => void
}

export const AuthContext = createContext<AuthContextValue | null>(null)

export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext)

  if (context === null) {
    throw new Error("useAuth must be used within an AuthProvider")
  }

  return context
}