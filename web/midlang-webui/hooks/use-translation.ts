import {
  type InfiniteData,
  type QueryKey,
  useInfiniteQuery,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query"
import type { components } from "@/api/generated"
import { useClient } from "@/hooks/use-client"
import type { SortOrder } from "@/lib/query"

export type TranslationListPage = components["schemas"]["TranslationListPage"]

type TranslationOptions = {
  readonly locale: string
  readonly keyword: string
  readonly order: SortOrder
  readonly key?: string
}

type SaveTranslationInput = {
  readonly locale: string
  readonly key: string
  readonly value: string
}

type DeleteTranslationInput = {
  readonly locale: string
  readonly key: string
}

export function useTranslation({ locale, keyword, order, key }: TranslationOptions) {
  const client = useClient()
  const queryClient = useQueryClient()

  const statisticsQuery = useQuery({
    queryKey: ["store-statistics", client],
    queryFn: async () => {
      const result = await client.GET("/store/statistics")
      return result.data!
    },
  })

  const locales = Object.keys(statisticsQuery.data?.per_locale ?? {})
  const selectedLocale = locale || locales[0] || ""

  const listQuery = useInfiniteQuery<
    TranslationListPage,
    Error,
    InfiniteData<TranslationListPage>,
    QueryKey,
    string | undefined
  >({
    queryKey: ["translations", client, selectedLocale, keyword, order],
    initialPageParam: undefined,
    getNextPageParam: (lastPage) => lastPage.next ?? undefined,
    enabled: selectedLocale !== "" && key === undefined,
    queryFn: async ({ pageParam }) => {
      const result = await client.GET("/t/{locale}", {
        params: {
          path: { locale: selectedLocale },
          query: {
            keyword: keyword || undefined,
            limit: 25,
            cursor: pageParam,
            order,
          },
        },
      })
      return result.data!
    },
  })

  const detailQuery = useQuery({
    queryKey: ["translation", client, selectedLocale, key],
    enabled: selectedLocale !== "" && key !== undefined,
    queryFn: async () => {
      if (key === undefined) throw new Error("translation key is required")
      const result = await client.GET("/t/{locale}/{key}", {
        params: { path: { locale: selectedLocale, key } },
      })
      return result.data!
    },
  })

  const invalidateTranslations = async () => {
    await queryClient.invalidateQueries({ queryKey: ["translations"] })
    await queryClient.invalidateQueries({ queryKey: ["store-statistics"] })
  }

  const saveMutation = useMutation({
    mutationFn: async ({ locale, key, value }: SaveTranslationInput) => {
      const result = await client.PUT("/t/{locale}/{key}", {
        params: { path: { locale, key } },
        body: { value },
      })
      return result.data!
    },
    onSuccess: invalidateTranslations,
  })

  const deleteMutation = useMutation({
    mutationFn: async ({ locale, key }: DeleteTranslationInput) => {
      await client.DELETE("/t/{locale}/{key}", {
        params: { path: { locale, key } },
      })
    },
    onSuccess: invalidateTranslations,
  })

  return {
    locales,
    selectedLocale,
    statisticsQuery,
    listQuery,
    detailQuery,
    saveMutation,
    deleteMutation,
  }
}
