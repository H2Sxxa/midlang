import { useMutation, useQueryClient } from "@tanstack/react-query"
import type { components } from "@/api/generated"
import { useClient } from "@/hooks/use-client"
import { useCursorPage } from "@/hooks/use-cursor-page"
import type { SortOrder } from "@/lib/query"

export type Issue = components["schemas"]["IssueMessage"]

export type IssueState = "open" | "closed" | "ignored"
export type IssueSort = "last_seen" | "created_at" | "count"
export type IssueKind = string

export const ISSUE_STATES: ReadonlyArray<IssueState> = ["open", "closed", "ignored"]
export const ISSUE_KINDS: ReadonlyArray<IssueKind> = ["missing_translation", "missing_locale"]

type IssueOptions = {
  readonly keyword: string
  readonly state: IssueState | ""
  readonly kind: IssueKind | ""
  readonly sort: IssueSort
  readonly order: SortOrder
  readonly cursor: string
  readonly onCursorChange: (cursor: string) => void
}

type SetStateInput = {
  readonly id: string
  readonly state: IssueState
}

/** One page of reported issues, plus the mutation that moves one through its states. */
export function useIssues({
  keyword,
  state,
  kind,
  sort,
  order,
  cursor,
  onCursorChange,
}: IssueOptions) {
  const client = useClient()
  const queryClient = useQueryClient()

  const list = useCursorPage<Issue>({
    queryKey: ["issues", client, keyword, state, kind, sort, order],
    enabled: true,
    cursor,
    onCursorChange,
    fetchPage: async (pageCursor) => {
      const result = await client.GET("/issues", {
        params: {
          query: {
            keyword: keyword || undefined,
            state: state || undefined,
            kind: kind || undefined,
            sort,
            order,
            limit: 25,
            cursor: pageCursor,
          },
        },
      })
      return result.data!
    },
  })

  const setStateMutation = useMutation({
    mutationFn: async ({ id, state: next }: SetStateInput) => {
      const result = await client.PATCH("/issues/{id}", {
        params: { path: { id } },
        body: { state: next },
      })
      return result.data!
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["issues"] })
    },
  })

  return { list, setStateMutation }
}
