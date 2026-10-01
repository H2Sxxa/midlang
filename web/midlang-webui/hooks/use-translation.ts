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
import { useToast } from "@/hooks/use-toast"

export type TranslationListPage = components["schemas"]["TranslationListPage"]
export type TranslationOrder = "asc" | "desc"

type TranslationOptions = {
  readonly locale: string
  readonly keyword: string
  readonly order: TranslationOrder
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
  const toast = useToast()

  const statisticsQuery = useQuery({
    queryKey: ["store-statistics", client],
    enabled: client !== null,
    queryFn: async () => {
      if (client === null) throw new Error("API client is unavailable")
      const result = await client.GET("/store/statistics")
      if (result.error !== undefined || result.data === undefined) {
        if (result.response.status === 401) {
          toast.error({ message: "Authentication expired", description: "Please sign in again." })
        }
        throw new Error("Unable to load locales.")
      }
      return result.data
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
    enabled: client !== null && selectedLocale !== "" && key === undefined,
    queryFn: async ({ pageParam }) => {
      if (client === null) throw new Error("API client is unavailable")
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
      if (result.error !== undefined || result.data === undefined) {
        if (result.response.status === 401) {
          toast.error({ message: "Authentication expired", description: "Please sign in again." })
        }
        throw new Error("Unable to load translations.")
      }
      return result.data
    },
  })

  const detailQuery = useQuery({
    queryKey: ["translation", client, selectedLocale, key],
    enabled: client !== null && selectedLocale !== "" && key !== undefined,
    queryFn: async () => {
      if (client === null || key === undefined) throw new Error("API client is unavailable")
      const result = await client.GET("/t/{locale}/{key}", {
        params: { path: { locale: selectedLocale, key } },
      })
      if (result.error !== undefined || result.data === undefined) {
        if (result.response.status === 401) {
          toast.error({ message: "Authentication expired", description: "Please sign in again." })
        }
        throw new Error("Unable to load translation.")
      }
      return result.data
    },
  })

  const saveMutation = useMutation({
    mutationFn: async ({ locale, key, value }: SaveTranslationInput) => {
      if (client === null) throw new Error("API client is unavailable")
      const result = await client.PUT("/t/{locale}/{key}", {
        params: { path: { locale, key } },
        body: { value },
      })
      if (result.error !== undefined) {
        if (result.response.status === 401) {
          toast.error({ message: "Authentication expired", description: "Please sign in again." })
        }
        throw new Error("Unable to save translation.")
      }
      return result.data
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["translations"] })
      await queryClient.invalidateQueries({ queryKey: ["store-statistics"] })
    },
  })

  const deleteMutation = useMutation({
    mutationFn: async ({ locale, key }: DeleteTranslationInput) => {
      if (client === null) throw new Error("API client is unavailable")
      const result = await client.DELETE("/t/{locale}/{key}", {
        params: { path: { locale, key } },
      })
      if (result.error !== undefined) {
        if (result.response.status === 401) {
          toast.error({ message: "Authentication expired", description: "Please sign in again." })
        }
        throw new Error("Unable to delete translation.")
      }
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["translations"] })
      await queryClient.invalidateQueries({ queryKey: ["store-statistics"] })
    },
  })

  return {
    client,
    locales,
    selectedLocale,
    statisticsQuery,
    listQuery,
    detailQuery,
    saveMutation,
    deleteMutation,
  }
}
