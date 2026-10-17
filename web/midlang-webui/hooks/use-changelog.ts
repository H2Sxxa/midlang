import { useMutation, useQueryClient } from "@tanstack/react-query"
import type { components } from "@/api/generated"
import { useClient } from "@/hooks/use-client"
import { useCursorPage } from "@/hooks/use-cursor-page"
import type { SortOrder } from "@/lib/query"

export type Change = components["schemas"]["ChangelogMessage"]

export type ChangeState = "created" | "updated" | "deleted"

export const CHANGE_STATES: ReadonlyArray<ChangeState> = ["created", "updated", "deleted"]

type ChangelogOptions = {
  readonly keyword: string
  readonly locale: string
  readonly state: ChangeState | ""
  readonly order: SortOrder
  readonly cursor: string
  readonly onCursorChange: (cursor: string) => void
}

/** One page of recorded changes, plus the mutation that rolls one back. */
export function useChangelog({
  keyword,
  locale,
  state,
  order,
  cursor,
  onCursorChange,
}: ChangelogOptions) {
  const client = useClient()
  const queryClient = useQueryClient()

  const list = useCursorPage<Change>({
    queryKey: ["changelog", client, keyword, locale, state, order],
    enabled: true,
    cursor,
    onCursorChange,
    fetchPage: async (pageCursor) => {
      const result = await client.GET("/changelog", {
        params: {
          query: {
            keyword: keyword || undefined,
            locale: locale || undefined,
            state: state || undefined,
            order,
            limit: 25,
            cursor: pageCursor,
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

  return { list, rollbackMutation }
}
