import { CaretRightIcon } from "@phosphor-icons/react"
import { Button } from "@/components/ui/button"

type LoadMoreProps = {
  readonly hasNextPage: boolean
  readonly isFetching: boolean
  readonly onLoadMore: () => void
}

/** Footer button shared by the paged lists. */
export function LoadMore({ hasNextPage, isFetching, onLoadMore }: LoadMoreProps) {
  return (
    <div className="mt-4 flex justify-center">
      <Button variant="outline" disabled={!hasNextPage || isFetching} onClick={onLoadMore}>
        {isFetching ? "Loading…" : "Load more"} <CaretRightIcon />
      </Button>
    </div>
  )
}
