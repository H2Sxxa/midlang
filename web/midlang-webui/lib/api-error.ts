import type { components } from "@/api/generated"

export type ApiErrorBody = components["schemas"]["ErrorBody"]

function isApiErrorBody(value: unknown): value is ApiErrorBody {
  if (typeof value !== "object" || value === null) {
    return false
  }
  return typeof (value as { message?: unknown }).message === "string"
}

/**
 * Turns an `openapi-fetch` error value into a message safe to show in the UI.
 * The server's `ErrorBody.message` wins, then any `Error`, then the fallback.
 */
export function describeApiError(error: unknown, fallback: string): string {
  if (isApiErrorBody(error)) {
    return error.message.length > 0 ? error.message : fallback
  }
  if (error instanceof Error && error.message.length > 0) {
    return error.message
  }
  return fallback
}
