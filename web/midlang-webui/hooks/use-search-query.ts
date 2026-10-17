import { useEffect, useRef, useState } from "react"

type UseSearchQueryOptions = {
  /** Value the list is filtered by right now, i.e. the one in the URL. */
  readonly applied: string
  readonly onApply: (value: string) => void
  readonly waitMs: number
}

export type SearchQuery = {
  /** Value bound to the input; trails `applied` while the user still types. */
  readonly draft: string
  readonly setDraft: (value: string) => void
}

/**
 * Couples a search input to the value it filters by. The input answers every
 * keystroke, and the applied value follows once typing pauses. A value changed
 * from the outside (back/forward, a shared link) drops any pending keystroke, so
 * the input never disagrees with the results on screen.
 */
export function useSearchQuery({ applied, onApply, waitMs }: UseSearchQueryOptions): SearchQuery {
  const [draft, setDraft] = useState(applied)
  const appliedRef = useRef(applied)
  const apply = useRef(onApply)
  const timer = useRef<number | null>(null)

  useEffect(() => {
    apply.current = onApply
  }, [onApply])

  useEffect(() => {
    if (applied === appliedRef.current) {
      return
    }
    if (timer.current !== null) {
      window.clearTimeout(timer.current)
      timer.current = null
    }
    appliedRef.current = applied
    setDraft(applied)
  }, [applied])

  useEffect(
    () => () => {
      if (timer.current !== null) {
        window.clearTimeout(timer.current)
      }
    },
    [],
  )

  const changeDraft = (value: string) => {
    setDraft(value)
    if (timer.current !== null) {
      window.clearTimeout(timer.current)
    }
    timer.current = window.setTimeout(() => {
      timer.current = null
      appliedRef.current = value
      apply.current(value)
    }, waitMs)
  }

  return { draft, setDraft: changeDraft }
}
