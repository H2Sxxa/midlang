import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import type { components } from "@/api/generated"
import { useClient } from "@/hooks/use-client"
import { useCursorPage, type CursorPageState } from "@/hooks/use-cursor-page"
import type { SortOrder } from "@/lib/query"

export type TranslationListEntry = components["schemas"]["TranslationListEntry"]

type TranslationOptions = {
  readonly locale: string
  readonly key?: string
}

type TranslationListOptions = {
  readonly locale: string
  readonly keyword: string
  readonly order: SortOrder
  readonly cursor: string
  readonly onCursorChange: (cursor: string) => void
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

/**
 * Write operations on translations. Kept apart from [useTranslation] so a page
 * that only writes does not also start the read queries, which fire on every
 * keystroke of a locale or key input.
 */
export function useTranslationMutations() {
  const client = useClient()
  const queryClient = useQueryClient()

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

  return { saveMutation, deleteMutation }
}

/** Locales the store holds, plus the one a list or detail view should open. */
export function useTranslation({ locale, key }: TranslationOptions) {
  const client = useClient()
  const { saveMutation, deleteMutation } = useTranslationMutations()

  const statisticsQuery = useQuery({
    queryKey: ["store-statistics", client],
    queryFn: async () => {
      const result = await client.GET("/store/statistics")
      return result.data!
    },
  })

  const locales = Object.keys(statisticsQuery.data?.per_locale ?? {})
  const selectedLocale = locale || locales[0] || ""

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

  return {
    locales,
    selectedLocale,
    statisticsQuery,
    detailQuery,
    saveMutation,
    deleteMutation,
  }
}

/**
 * One page of the translations of a locale. The cursor lives in the URL, so a
 * page is linkable and stepping back uses the cursor the endpoint handed back.
 */
export function useTranslationList({
  locale,
  keyword,
  order,
  cursor,
  onCursorChange,
}: TranslationListOptions): CursorPageState<TranslationListEntry> {
  const client = useClient()

  return useCursorPage<TranslationListEntry>({
    queryKey: ["translations", client, locale, keyword, order],
    enabled: locale !== "",
    cursor,
    onCursorChange,
    fetchPage: async (pageCursor) => {
      const result = await client.GET("/t/{locale}", {
        params: {
          path: { locale },
          query: {
            keyword: keyword || undefined,
            limit: 25,
            cursor: pageCursor,
            order,
          },
        },
      })
      return result.data!
    },
  })
}
