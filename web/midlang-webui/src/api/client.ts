import createClient from "openapi-fetch"
import type { paths } from "./generated"

export function createApiClient(remoteUrl: string, token: string) {
  return createClient<paths>({
    baseUrl: remoteUrl.replace(/\/$/, ""),
    headers: {
      Authorization: `Bearer ${token}`,
    },
  })
}

export type ApiClient = ReturnType<typeof createApiClient>
