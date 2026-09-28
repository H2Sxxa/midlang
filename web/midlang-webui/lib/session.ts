export type StoredSession = {
  readonly token: string
  readonly remoteUrl: string
}

const SESSION_STORAGE_KEY = "midlang.session"

function isStoredSession(value: unknown): value is StoredSession {
  if (typeof value !== "object" || value === null) {
    return false
  }

  const candidate = value as Record<string, unknown>
  return typeof candidate.token === "string" && typeof candidate.remoteUrl === "string"
}

export function readSession(): StoredSession | null {
  const raw = window.localStorage.getItem(SESSION_STORAGE_KEY)

  if (raw === null) {
    return null
  }

  let parsed: unknown
  try {
    parsed = JSON.parse(raw)
  } catch (error) {
    console.warn("Discarding unreadable stored session", { key: SESSION_STORAGE_KEY, error })
    window.localStorage.removeItem(SESSION_STORAGE_KEY)
    return null
  }

  if (!isStoredSession(parsed)) {
    console.warn("Discarding stored session with an unexpected shape", {
      key: SESSION_STORAGE_KEY,
      session: parsed,
    })
    window.localStorage.removeItem(SESSION_STORAGE_KEY)
    return null
  }

  return parsed
}

export function writeSession(session: StoredSession): void {
  window.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(session))
}

export function clearSession(): void {
  window.localStorage.removeItem(SESSION_STORAGE_KEY)
}