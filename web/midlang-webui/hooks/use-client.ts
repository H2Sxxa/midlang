import { useMemo } from "react"
import { createApiClient, type ApiClient } from "@/api/client"
import { useAuth } from "@/hooks/use-auth"

export function useClient(): ApiClient | null {
  const { remoteUrl, token } = useAuth()

  return useMemo(
    () => (token === null ? null : createApiClient(remoteUrl, token)),
    [remoteUrl, token],
  )
}
