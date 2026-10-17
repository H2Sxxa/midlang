import { keepPreviousData, useQuery, type QueryKey } from "@tanstack/react-query"
import { useState } from "react"
import { FIRST_PAGE } from "@/lib/query"

/** One page of a cursor-paged list endpoint. */
export type CursorPage<TItem> = {
  readonly items: ReadonlyArray<TItem>
  readonly next?: string | null
}

type UseCursorPageOptions<TItem> = {
  readonly queryKey: QueryKey
  readonly enabled: boolean
  /** Cursor of the page to read, as it travels in the URL. */
  readonly cursor: string
  readonly fetchPage: (cursor: string | undefined) => Promise<CursorPage<TItem>>
  readonly onCursorChange: (cursor: string) => void
}

export type CursorPageState<TItem> = {
  readonly items: ReadonlyArray<TItem>
  readonly hasPrev: boolean
  readonly hasNext: boolean
  readonly isFirst: boolean
  readonly isLoading: boolean
  readonly isFetching: boolean
  readonly isError: boolean
  readonly error: Error | null
  readonly goNext: () => void
  readonly goPrev: () => void
  readonly goFirst: () => void
}

/**
 * Reads one page of a cursor-paged endpoint.
 *
 * A cursor names a place, not a step, so the endpoint only ever hands back the
 * place the next page starts at: the pages a reader walked through are the
 * reader's own trail. The hook keeps that trail - oldest first, ending at the
 * page on screen - which is what stepping back walks. A page opened from a link
 * starts a trail of its own, so its control steps back only as far as the link
 * itself, never into a session that reached it some other way.
 *
 * The cursor is part of the query key, so every visited page stays cached and
 * stepping back is instant, and the page on screen stays visible while the next
 * one loads.
 */
export function useCursorPage<TItem>({
  queryKey,
  enabled,
  cursor,
  fetchPage,
  onCursorChange,
}: UseCursorPageOptions<TItem>): CursorPageState<TItem> {
  const query = useQuery({
    queryKey: [...queryKey, cursor],
    queryFn: () => fetchPage(cursor === FIRST_PAGE ? undefined : cursor),
    enabled,
    placeholderData: keepPreviousData,
  })
  const next = query.data?.next ?? null

  // The trail follows the cursor the URL carries: a cursor already in the trail
  // cuts it back to that place - a step back, or the browser's own history - and
  // one the trail has never seen starts a fresh trail with itself. Adjusting
  // here rather than in an effect keeps the trail in step with the page on
  // screen within the same render.
  const [trail, setTrail] = useState<ReadonlyArray<string>>(() => [cursor])
  const [readCursor, setReadCursor] = useState(cursor)
  if (readCursor !== cursor) {
    setReadCursor(cursor)
    setTrail((current) => {
      const place = current.lastIndexOf(cursor)
      return place === -1 ? [cursor] : current.slice(0, place + 1)
    })
  }

  const back = trail.length > 1 ? trail[trail.length - 2] : null

  return {
    items: query.data?.items ?? [],
    hasPrev: back !== null,
    hasNext: next !== null,
    isFirst: cursor === FIRST_PAGE,
    isLoading: query.isLoading,
    isFetching: query.isFetching,
    isError: query.isError,
    error: query.error,
    goNext: () => {
      if (next !== null) {
        setTrail((current) => [...current, next])
        onCursorChange(next)
      }
    },
    goPrev: () => {
      if (back !== null) {
        setTrail((current) => current.slice(0, -1))
        onCursorChange(back)
      }
    },
    goFirst: () => {
      setTrail([FIRST_PAGE])
      onCursorChange(FIRST_PAGE)
    },
  }
}
