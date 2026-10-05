import type { components } from "@/api/generated"

export type ApiErrorBody = components["schemas"]["ErrorBody"]

/**
 * Failure of an API request. Carries the HTTP status and the server error code
 * so the global error handler can tell an expired session from anything else.
 */
export class ApiError extends Error {
  readonly status: number
  readonly code: string

  constructor(status: number, code: string, message: string) {
    super(message)
    this.name = "ApiError"
    this.status = status
    this.code = code
  }
}

function isApiErrorBody(value: unknown): value is ApiErrorBody {
  if (typeof value !== "object" || value === null) {
    return false
  }
  return typeof (value as { message?: unknown }).message === "string"
}

/**
 * Turns an `openapi-fetch` failure into an `ApiError`. The server's
 * `ErrorBody.message` wins, then any `Error`, then a status-based fallback.
 */
export function apiErrorFrom(error: unknown, response: Response): ApiError {
  if (isApiErrorBody(error)) {
    return new ApiError(response.status, error.code, error.message)
  }
  if (error instanceof Error && error.message.length > 0) {
    return new ApiError(response.status, "request_failed", error.message)
  }
  return new ApiError(
    response.status,
    "request_failed",
    `request failed with status ${response.status}`,
  )
}

/** True when the failure means the bearer token is missing, invalid or expired. */
export function isUnauthorized(error: unknown): boolean {
  return error instanceof ApiError && error.status === 401
}

/** Message safe to show for any thrown value. */
export function errorMessage(error: unknown): string {
  if (error instanceof Error && error.message.length > 0) {
    return error.message
  }
  return String(error)
}
