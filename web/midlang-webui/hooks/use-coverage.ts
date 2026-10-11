import {
  type InfiniteData,
  type QueryKey,
  useInfiniteQuery,
  useQuery,
} from "@tanstack/react-query"
import type { components } from "@/api/generated"
import { useClient } from "@/hooks/use-client"

export type CoverageReport = components["schemas"]["CoverageMessage"]
export type LocaleCoverage = components["schemas"]["LocaleCoverageMessage"]
export type MissingKeysPage = components["schemas"]["MissingKeysPage"]

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

/** Paged keys `locale` does not translate; skipped while either locale is empty. */
export function useMissingKeys(reference: string, locale: string) {
  const client = useClient()

  return useInfiniteQuery<
    MissingKeysPage,
    Error,
    InfiniteData<MissingKeysPage>,
    QueryKey,
    string | undefined
  >({
    queryKey: ["coverage-missing", client, reference, locale],
    enabled: reference !== "" && locale !== "",
    initialPageParam: undefined,
    getNextPageParam: (lastPage) => lastPage.next ?? undefined,
    queryFn: async ({ pageParam }) => {
      const result = await client.GET("/coverage/{reference}/{locale}/missing", {
        params: {
          path: { reference, locale },
          query: { limit: 50, cursor: pageParam },
        },
      })
      return result.data!
    },
  })
}
