import {
  type InfiniteData,
  type QueryKey,
  useInfiniteQuery,
  useMutation,
  useQueryClient,
} from "@tanstack/react-query"
import type { components } from "@/api/generated"
import { useClient } from "@/hooks/use-client"
import type { SortOrder } from "@/lib/query"

export type Issue = components["schemas"]["IssueMessage"]
export type IssueListPage = components["schemas"]["IssueListPage"]

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
}

type SetStateInput = {
  readonly id: string
  readonly state: IssueState
}

export function useIssues({ keyword, state, kind, sort, order }: IssueOptions) {
  const client = useClient()
  const queryClient = useQueryClient()

  const listQuery = useInfiniteQuery<
    IssueListPage,
    Error,
    InfiniteData<IssueListPage>,
    QueryKey,
    string | undefined
  >({
    queryKey: ["issues", client, keyword, state, kind, sort, order],
    initialPageParam: undefined,
    getNextPageParam: (lastPage) => lastPage.next ?? undefined,
    queryFn: async ({ pageParam }) => {
      const result = await client.GET("/issues", {
        params: {
          query: {
            keyword: keyword || undefined,
            state: state || undefined,
            kind: kind || undefined,
            sort,
            order,
            limit: 25,
            cursor: pageParam,
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

  return { listQuery, setStateMutation }
}
