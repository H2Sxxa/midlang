import { PencilSimpleIcon, PlusIcon } from "@phosphor-icons/react"
import { createFileRoute, Link } from "@tanstack/react-router"
import { CursorPagination } from "@/components/cursor-pagination"
import { DataTableCard, type TableState } from "@/components/data-table-card"
import { OrderButton } from "@/components/order-button"
import { SearchInput } from "@/components/search-input"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { useAuth } from "@/hooks/use-auth"
import { useSearchQuery } from "@/hooks/use-search-query"
import { useTranslation, useTranslationList } from "@/hooks/use-translation"
import { FIRST_PAGE, SORT_ORDERS, parseChoice, parseText, type SortOrder } from "@/lib/query"

/** How long typing pauses before a text filter applies. */
const SEARCH_WAIT_MS = 300

/**
 * List state lives in the URL, so a page is linkable, survives a refresh and
 * steps with back/forward. Every field is optional, which is what lets a plain
 * link to the list open it on its first unfiltered page.
 */
type TranslationSearch = {
  readonly q?: string
  readonly locale?: string
  readonly order?: SortOrder
  readonly cursor?: string
}

/** Reads the list params, filling in whatever the URL leaves out. */
function resolveTranslationSearch(search: Record<string, unknown>): Required<TranslationSearch> {
  return {
    q: parseText(search.q),
    locale: parseText(search.locale),
    order: parseChoice(search.order, SORT_ORDERS, "asc"),
    cursor: parseText(search.cursor),
  }
}

export const Route = createFileRoute("/_app/translations/")({
  // The route's search type stays optional, so a link may omit every param.
  validateSearch: (search: Record<string, unknown>): TranslationSearch =>
    resolveTranslationSearch(search),
  component: TranslationListPage,
})

function TranslationListPage() {
  const auth = useAuth()
  const navigate = Route.useNavigate()
  const search = resolveTranslationSearch(Route.useSearch())
  const translation = useTranslation({ locale: search.locale })
  const { locales, selectedLocale } = translation

  const changeFilters = (patch: Partial<TranslationSearch>) => {
    void navigate({ search: (prev) => ({ ...prev, ...patch, cursor: undefined }) })
  }

  /** Opens the page a cursor points at; the first page drops the param. */
  const changeCursor = (cursor: string) => {
    void navigate({
      search: (prev) => ({ ...prev, cursor: cursor === FIRST_PAGE ? undefined : cursor }),
    })
  }

  const keywordQuery = useSearchQuery({
    applied: search.q,
    onApply: (value) => {
      void navigate({
        search: (prev) => ({ ...prev, q: value, cursor: undefined }),
        replace: true,
      })
    },
    waitMs: SEARCH_WAIT_MS,
  })

  const localeQuery = useSearchQuery({
    applied: search.locale,
    onApply: (value) => {
      void navigate({
        search: (prev) => ({ ...prev, locale: value, cursor: undefined }),
        replace: true,
      })
    },
    waitMs: SEARCH_WAIT_MS,
  })

  const list = useTranslationList({
    locale: selectedLocale,
    keyword: search.q.trim(),
    order: search.order,
    cursor: search.cursor,
    onCursorChange: changeCursor,
  })

  const tableState: TableState = list.isError
    ? { kind: "error", message: list.error?.message ?? "Unable to load translations." }
    : list.isLoading
      ? { kind: "loading", message: "Loading translations…" }
      : list.items.length === 0
        ? {
            kind: "empty",
            message:
              selectedLocale === ""
                ? "Create a translation to add the first locale."
                : "No matching translations.",
          }
        : { kind: "ready" }

  const pager = (
    <CursorPagination
      isFirst={list.isFirst}
      hasPrev={list.hasPrev}
      hasNext={list.hasNext}
      isFetching={list.isFetching}
      onFirst={list.goFirst}
      onPrev={list.goPrev}
      onNext={list.goNext}
    />
  )

  return (
    <div className="space-y-6">
      <div className="flex items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Translation List</h1>
          <p className="mt-1 text-muted-foreground">Search and browse translation resources.</p>
        </div>
        {auth.can("translation:write") && (
          <Button nativeButton={false} render={<Link to="/translations/new" />}>
            <PlusIcon /> Add translation
          </Button>
        )}
      </div>

      <Card>
        <CardContent className="flex flex-col gap-3 pt-6 md:flex-row md:items-end">
          <div className="flex flex-1 flex-col gap-1.5">
            <Label htmlFor="locale">Locale</Label>
            {locales.length === 0 ? (
              <Input
                id="locale"
                placeholder="en-US"
                value={localeQuery.draft}
                onChange={(event) => localeQuery.setDraft(event.target.value)}
              />
            ) : (
              <Select
                value={selectedLocale}
                onValueChange={(next) => {
                  if (next !== null) {
                    changeFilters({ locale: next })
                  }
                }}
              >
                <SelectTrigger id="locale" className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {locales.map((item) => (
                    <SelectItem key={item} value={item}>
                      {item}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          </div>
          <SearchInput
            id="keyword"
            label="Search translations"
            placeholder="Search keys or values"
            value={keywordQuery.draft}
            onChange={keywordQuery.setDraft}
          />
          <OrderButton
            id="order"
            order={search.order}
            onChange={(order) => changeFilters({ order })}
          />
        </CardContent>
      </Card>

      <DataTableCard
        title={selectedLocale || "Translation entries"}
        description={
          search.q ? `Results matching “${search.q}”.` : "All entries for the selected locale."
        }
        state={tableState}
        footer={pager}
      >
        <div className="rounded-md border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Key</TableHead>
                <TableHead>Value</TableHead>
                <TableHead className="w-20 text-right">Edit</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {list.items.map((item) => (
                <TableRow key={item.key}>
                  <TableCell className="font-medium">{item.key}</TableCell>
                  <TableCell className="max-w-xl whitespace-normal">{item.value}</TableCell>
                  <TableCell className="text-right">
                    {auth.can("translation:read") && (
                      <Button
                        nativeButton={false}
                        size="icon-sm"
                        variant="ghost"
                        aria-label={`Edit ${item.key}`}
                        render={
                          <Link
                            to="/translations/$locale/$key"
                            params={{ locale: selectedLocale, key: item.key }}
                          />
                        }
                      >
                        <PencilSimpleIcon />
                      </Button>
                    )}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      </DataTableCard>
    </div>
  )
}
