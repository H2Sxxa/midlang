import { useState, type ReactNode } from "react"
import { defaultRemoteUrl } from "@/hooks/config"
import { AuthContext, type AuthStatus } from "@/hooks/use-auth"
import { clearSession, readSession, writeSession } from "@/lib/session"

type AuthState = {
  readonly status: AuthStatus
  readonly token: string | null
  readonly remoteUrl: string
}

type AuthProviderProps = {
  readonly children: ReactNode
}

function createInitialState(): AuthState {
  const session = readSession()

  if (session === null) {
    return { status: "unauthenticated", token: null, remoteUrl: defaultRemoteUrl }
  }

  return { status: "authenticated", token: session.token, remoteUrl: session.remoteUrl }
}

export function AuthProvider({ children }: AuthProviderProps) {
  const [state, setState] = useState<AuthState>(createInitialState)

  const login = async (token: string, remoteUrl: string): Promise<void> => {
    setState((current) => ({ ...current, status: "authenticating" }))

    try {
      // TODO: verify the token against remoteUrl before trusting it, once the server
      // exposes an authentication endpoint.
      writeSession({ token, remoteUrl })
      setState({ status: "authenticated", token, remoteUrl })
    } catch (error) {
      setState((current) => ({ ...current, status: "error" }))
      throw error
    }
  }

  const logout = (): void => {
    clearSession()
    setState((current) => ({
      status: "unauthenticated",
      token: null,
      remoteUrl: current.remoteUrl,
    }))
  }

  return (
    <AuthContext.Provider
      value={{
        status: state.status,
        token: state.token,
        remoteUrl: state.remoteUrl,
        login,
        logout,
      }}
    >
      {children}
    </AuthContext.Provider>
  )
}