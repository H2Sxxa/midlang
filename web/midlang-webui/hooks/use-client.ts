import { useMemo } from "react"
import { createApiClient, type ApiClient } from "@/api/client"
import { useAuth } from "@/hooks/use-auth"

/**
 * Authenticated API client. Every caller lives behind the `_app` auth guard, so
 * a missing token is a programming error rather than a state to render.
 */
export function useClient(): ApiClient {
  const { remoteUrl, token } = useAuth()

  const client = useMemo(
    () => (token === null ? null : createApiClient(remoteUrl, token)),
    [remoteUrl, token],
  )

  if (client === null) {
    throw new Error("useClient must be used within an authenticated session")
  }

  return client
}
