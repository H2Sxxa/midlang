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

export type Change = components["schemas"]["ChangelogMessage"]
export type ChangeListPage = components["schemas"]["ChangelogListPage"]

export type ChangeState = "created" | "updated" | "deleted"

export const CHANGE_STATES: ReadonlyArray<ChangeState> = ["created", "updated", "deleted"]

type ChangelogOptions = {
  readonly keyword: string
  readonly locale: string
  readonly state: ChangeState | ""
  readonly order: SortOrder
}

export function useChangelog({ keyword, locale, state, order }: ChangelogOptions) {
  const client = useClient()
  const queryClient = useQueryClient()

  const listQuery = useInfiniteQuery<
    ChangeListPage,
    Error,
    InfiniteData<ChangeListPage>,
    QueryKey,
    string | undefined
  >({
    queryKey: ["changelog", client, keyword, locale, state, order],
    initialPageParam: undefined,
    getNextPageParam: (lastPage) => lastPage.next ?? undefined,
    queryFn: async ({ pageParam }) => {
      const result = await client.GET("/changelog", {
        params: {
          query: {
            keyword: keyword || undefined,
            locale: locale || undefined,
            state: state || undefined,
            order,
            limit: 25,
            cursor: pageParam,
          },
        },
      })
      return result.data!
    },
  })

  const rollbackMutation = useMutation({
    mutationFn: async (id: number) => {
      const result = await client.POST("/changelog/{id}/rollback", {
        params: { path: { id } },
      })
      return result.data!
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["changelog"] })
    },
  })

  return { listQuery, rollbackMutation }
}
