import createClient, { type Middleware } from "openapi-fetch"
import { apiErrorFrom } from "@/lib/api-error"
import type { paths } from "./generated"

/**
 * Turns every failed response into a thrown `ApiError`, so request code never
 * inspects `response.ok` or `result.error` itself: a call that resolves always
 * carries data, which is why call sites can read `result.data` directly.
 */
const raiseApiErrors: Middleware = {
  async onResponse({ response }) {
    if (response.ok) {
      return
    }
    const error = await response.clone().json().catch(() => undefined)
    throw apiErrorFrom(error, response)
  },
}

export function createApiClient(remoteUrl: string, token: string) {
  const client = createClient<paths>({
    baseUrl: remoteUrl.replace(/\/$/, ""),
    headers: {
      Authorization: `Bearer ${token}`,
    },
  })
  client.use(raiseApiErrors)
  return client
}

export type ApiClient = ReturnType<typeof createApiClient>
