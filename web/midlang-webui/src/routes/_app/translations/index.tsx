import { MagnifyingGlassIcon, PencilSimpleIcon, PlusIcon } from "@phosphor-icons/react"
import { createFileRoute, Link } from "@tanstack/react-router"
import { useState, type SubmitEvent } from "react"
import { LoadMore } from "@/components/load-more"
import { OrderButton } from "@/components/order-button"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { useAuth } from "@/hooks/use-auth"
import { useTranslation } from "@/hooks/use-translation"
import type { SortOrder } from "@/lib/query"

export const Route = createFileRoute("/_app/translations/")({
  component: TranslationListPage,
})

function TranslationListPage() {
  const auth = useAuth()
  const [locale, setLocale] = useState("")
  const [localeDraft, setLocaleDraft] = useState("")
  const [keyword, setKeyword] = useState("")
  const [appliedKeyword, setAppliedKeyword] = useState("")
  const [order, setOrder] = useState<SortOrder>("asc")
  const translation = useTranslation({ locale, keyword: appliedKeyword, order })
  const { locales, selectedLocale } = translation
  const entries = translation.listQuery.data?.pages.flatMap((page) => page.items) ?? []

  const handleSearch = (event: SubmitEvent) => {
    event.preventDefault()
    setAppliedKeyword(keyword.trim())
    setLocale(localeDraft.trim())
  }

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
                value={localeDraft}
                onChange={(event) => setLocaleDraft(event.target.value)}
              />
            ) : (
              <Select
                value={selectedLocale}
                onValueChange={(next) => {
                  if (next !== null) {
                    setLocale(next)
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
          <form className="flex flex-2 gap-2" onSubmit={handleSearch}>
            <div className="relative flex-1">
              <MagnifyingGlassIcon className="absolute top-2 left-2.5 size-4 text-muted-foreground" />
              <Input
                className="pl-8"
                placeholder="Search keys or values"
                value={keyword}
                onChange={(event) => setKeyword(event.target.value)}
              />
            </div>
            <Button type="submit" variant="outline">
              Search
            </Button>
          </form>
          <OrderButton id="order" order={order} onChange={setOrder} />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{selectedLocale || "Translation entries"}</CardTitle>
          <CardDescription>
            {appliedKeyword
              ? `Results matching “${appliedKeyword}”.`
              : "All entries for the selected locale."}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {translation.listQuery.isError ? (
            <p className="py-8 text-sm text-destructive">{translation.listQuery.error.message}</p>
          ) : translation.listQuery.isLoading ? (
            <p className="py-8 text-sm text-muted-foreground">Loading translations…</p>
          ) : selectedLocale === "" ? (
            <p className="py-8 text-sm text-muted-foreground">
              Create a translation to add the first locale.
            </p>
          ) : (
            <>
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
                    {entries.map((item) => (
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
                {entries.length === 0 && (
                  <p className="px-4 py-8 text-sm text-muted-foreground">No matching translations.</p>
                )}
              </div>
              <LoadMore
                hasNextPage={translation.listQuery.hasNextPage}
                isFetching={translation.listQuery.isFetchingNextPage}
                onLoadMore={() => void translation.listQuery.fetchNextPage()}
              />
            </>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
