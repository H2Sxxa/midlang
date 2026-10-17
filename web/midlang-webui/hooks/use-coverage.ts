import { useQuery } from "@tanstack/react-query"
import type { components } from "@/api/generated"
import { useClient } from "@/hooks/use-client"
import { useCursorPage, type CursorPageState } from "@/hooks/use-cursor-page"

export type CoverageReport = components["schemas"]["CoverageMessage"]
export type LocaleCoverage = components["schemas"]["LocaleCoverageMessage"]

/** Locales coverage knows about, used to pick the locale it is measured against. */
export function useCoverageLocales() {
  const client = useClient()

  const localesQuery = useQuery({
    queryKey: ["coverage-locales", client],
    queryFn: async () => (await client.GET("/coverage/locales")).data!,
  })

  return {
    localesQuery,
    locales: localesQuery.data?.locales ?? [],
  }
}

/** Coverage of every other locale against `reference`; skipped while it is empty. */
export function useCoverageReport(reference: string) {
  const client = useClient()

  return useQuery({
    queryKey: ["coverage", client, reference],
    enabled: reference !== "",
    queryFn: async () => {
      const result = await client.GET("/coverage/{reference}", {
        params: { path: { reference } },
      })
      return result.data!
    },
  })
}

type MissingKeysOptions = {
  readonly reference: string
  readonly locale: string
  readonly cursor: string
  readonly onCursorChange: (cursor: string) => void
}

/** Keys `locale` does not translate; skipped while either locale is empty. */
export function useMissingKeys({
  reference,
  locale,
  cursor,
  onCursorChange,
}: MissingKeysOptions): CursorPageState<string> {
  const client = useClient()

  return useCursorPage<string>({
    queryKey: ["coverage-missing", client, reference, locale],
    enabled: reference !== "" && locale !== "",
    cursor,
    onCursorChange,
    fetchPage: async (pageCursor) => {
      const result = await client.GET("/coverage/{reference}/{locale}/missing", {
        params: {
          path: { reference, locale },
          query: { limit: 50, cursor: pageCursor },
        },
      })
      return result.data!
    },
  })
}
