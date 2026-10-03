/** Formats Unix seconds as a local date-time, or a dash when absent. */
export function formatTimestamp(epochSeconds: number | null | undefined): string {
  if (epochSeconds === null || epochSeconds === undefined) {
    return "—"
  }
  return new Date(epochSeconds * 1000).toLocaleString()
}

/** Converts Unix seconds to the value a `datetime-local` input expects. */
export function epochSecondsToLocalInput(epochSeconds: number | null | undefined): string {
  if (epochSeconds === null || epochSeconds === undefined) {
    return ""
  }
  const date = new Date(epochSeconds * 1000)
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000)
  return local.toISOString().slice(0, 16)
}

/** Converts a `datetime-local` value to Unix seconds, or null when empty or invalid. */
export function localInputToEpochSeconds(value: string): number | null {
  if (value === "") {
    return null
  }
  const milliseconds = new Date(value).getTime()
  if (Number.isNaN(milliseconds)) {
    return null
  }
  return Math.floor(milliseconds / 1000)
}
