import { useState, type ReactNode } from "react"
import { createApiClient } from "@/api/client"
import { defaultRemoteUrl } from "@/hooks/config"
import { useToast } from "@/hooks/use-toast"
import { AuthContext, type AuthStatus, type Permission } from "@/hooks/use-auth"
import { errorMessage, isUnauthorized } from "@/lib/api-error"

type AuthState = {
  readonly status: AuthStatus
  readonly token: string | null
  readonly remoteUrl: string
  readonly permissions: ReadonlyArray<Permission>
}

type AuthProviderProps = {
  readonly children: ReactNode
}

function createInitialState(): AuthState {
  return {
    status: "unauthenticated",
    token: null,
    remoteUrl: defaultRemoteUrl,
    permissions: [],
  }
}

export function AuthProvider({ children }: AuthProviderProps) {
  const [state, setState] = useState<AuthState>(createInitialState)
  const toast = useToast()

  const login = async (token: string, remoteUrl: string): Promise<void> => {
    setState((current) => ({ ...current, status: "authenticating" }))

    try {
      const client = createApiClient(remoteUrl, token)
      const result = await client.GET("/auth/permissions")

      setState({
        status: "authenticated",
        token,
        remoteUrl,
        permissions: result.data!.permissions,
      })
    } catch (error) {
      setState((current) => ({ ...current, status: "error" }))
      toast.error({
        message: "Sign in failed",
        description: isUnauthorized(error)
          ? "The API token is invalid or expired."
          : errorMessage(error),
      })
      throw error
    }
  }

  const logout = (): void => {
    setState((current) => ({
      status: "unauthenticated",
      token: null,
      remoteUrl: current.remoteUrl,
      permissions: [],
    }))
  }

  return (
    <AuthContext.Provider
      value={{
        status: state.status,
        token: state.token,
        remoteUrl: state.remoteUrl,
        permissions: state.permissions,
        login,
        logout,
        can: (permission) => state.permissions.includes(permission),
      }}
    >
      {children}
    </AuthContext.Provider>
  )
}
