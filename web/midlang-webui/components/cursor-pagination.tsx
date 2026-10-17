import { CaretDoubleLeftIcon, CaretLeftIcon, CaretRightIcon } from "@phosphor-icons/react"
import { Button } from "@/components/ui/button"
import { Pagination, PaginationContent, PaginationItem } from "@/components/ui/pagination"

type CursorPaginationProps = {
  readonly isFirst: boolean
  readonly hasPrev: boolean
  readonly hasNext: boolean
  readonly isFetching: boolean
  readonly onFirst: () => void
  readonly onPrev: () => void
  readonly onNext: () => void
}

/**
 * Pager for cursor-paged lists.
 *
 * The endpoint returns a cursor rather than a total, so there is no page count
 * to show and no page to jump to: `Next` walks the token the page on screen
 * handed back, `Previous` walks the trail the reader's own session kept, and
 * `First` returns to the start of the listing. It renders nothing when the
 * listing is on its only page, so a caller can place it unconditionally.
 */
export function CursorPagination({
  isFirst,
  hasPrev,
  hasNext,
  isFetching,
  onFirst,
  onPrev,
  onNext,
}: CursorPaginationProps) {
  if (isFirst && !hasPrev && !hasNext) {
    return null
  }
  return (
    <Pagination className="mt-4">
      <PaginationContent>
        <PaginationItem>
          <Button variant="outline" size="sm" disabled={isFirst || isFetching} onClick={onFirst}>
            <CaretDoubleLeftIcon data-icon="inline-start" /> First
          </Button>
        </PaginationItem>
        <PaginationItem>
          <Button variant="outline" size="sm" disabled={!hasPrev || isFetching} onClick={onPrev}>
            <CaretLeftIcon data-icon="inline-start" /> Previous
          </Button>
        </PaginationItem>
        <PaginationItem>
          <span className="w-20 text-center text-xs text-muted-foreground" aria-live="polite">
            {isFetching ? "Loading…" : ""}
          </span>
        </PaginationItem>
        <PaginationItem>
          <Button variant="outline" size="sm" disabled={!hasNext || isFetching} onClick={onNext}>
            Next <CaretRightIcon data-icon="inline-end" />
          </Button>
        </PaginationItem>
      </PaginationContent>
    </Pagination>
  )
}
