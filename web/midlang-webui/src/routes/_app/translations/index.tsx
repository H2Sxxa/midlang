import {
  CaretRightIcon,
  MagnifyingGlassIcon,
  PencilSimpleIcon,
  PlusIcon,
} from "@phosphor-icons/react"
import { createFileRoute, Link } from "@tanstack/react-router"
import { useState, type SubmitEvent } from "react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { useTranslation, type TranslationOrder } from "@/hooks/use-translation"
import { useAuth } from "@/hooks/use-auth"

export const Route = createFileRoute("/_app/translations/")({
  component: TranslationListPage,
})

function TranslationListPage() {
  const auth = useAuth()
  const [locale, setLocale] = useState("")
  const [keyword, setKeyword] = useState("")
  const [appliedKeyword, setAppliedKeyword] = useState("")
  const [order, setOrder] = useState<TranslationOrder>("asc")
  const translation = useTranslation({ locale, keyword: appliedKeyword, order })
  const { locales, selectedLocale } = translation
  const entries = translation.listQuery.data?.pages.flatMap((page) => page.items) ?? []

  const handleSearch = (event: SubmitEvent) => {
    event.preventDefault()
    setAppliedKeyword(keyword.trim())
  }

  return (
    <div className="space-y-6">
      <div className="flex items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Translation List</h1>
          <p className="mt-1 text-muted-foreground">Search and browse translation resources.</p>
        </div>
        {auth.can("translation:write") && (
          <Button nativeButton={false} render={<Link to="/translations/new" />}><PlusIcon /> Add translation</Button>
        )}
      </div>

      <Card>
        <CardContent className="flex flex-col gap-3 pt-6 md:flex-row md:items-end">
          <div className="flex flex-1 flex-col gap-1.5">
            <Label htmlFor="locale">Locale</Label>
            {locales.length === 0 ? (
              <Input id="locale" placeholder="en-US" value={locale} onChange={(event) => setLocale(event.target.value)} />
            ) : (
              <select
                id="locale"
                className="h-8 border border-input bg-transparent px-2.5 text-xs outline-none focus:border-ring focus:ring-1 focus:ring-ring/50"
                value={selectedLocale}
                onChange={(event) => setLocale(event.target.value)}
              >
                {locales.map((item) => <option key={item}>{item}</option>)}
              </select>
            )}
          </div>
          <form className="flex flex-2 gap-2" onSubmit={handleSearch}>
            <div className="relative flex-1">
              <MagnifyingGlassIcon className="absolute top-2 left-2.5 size-4 text-muted-foreground" />
              <Input className="pl-8" placeholder="Search keys or values" value={keyword} onChange={(event) => setKeyword(event.target.value)} />
            </div>
            <Button type="submit" variant="outline">Search</Button>
          </form>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="order">Order</Label>
            <select
              id="order"
              className="h-8 border border-input bg-transparent px-2.5 text-xs outline-none focus:border-ring focus:ring-1 focus:ring-ring/50"
              value={order}
              onChange={(event) => setOrder(event.target.value as TranslationOrder)}
            >
              <option value="asc">A → Z</option>
              <option value="desc">Z → A</option>
            </select>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{selectedLocale || "Translation entries"}</CardTitle>
          <CardDescription>{appliedKeyword ? `Results matching “${appliedKeyword}”.` : "All entries for the selected locale."}</CardDescription>
        </CardHeader>
        <CardContent>
          {translation.listQuery.isError ? (
            <p className="py-8 text-sm text-destructive">{translation.listQuery.error.message}</p>
          ) : translation.listQuery.isLoading ? (
            <p className="py-8 text-sm text-muted-foreground">Loading translations…</p>
          ) : selectedLocale === "" ? (
            <p className="py-8 text-sm text-muted-foreground">Create a translation to add the first locale.</p>
          ) : (
            <>
              <div className="rounded-md border">
                <Table>
                  <TableHeader><TableRow><TableHead>Key</TableHead><TableHead>Value</TableHead><TableHead className="w-20 text-right">Edit</TableHead></TableRow></TableHeader>
                  <TableBody>
                    {entries.map((item) => (
                      <TableRow key={item.key}>
                        <TableCell className="font-medium">{item.key}</TableCell>
                        <TableCell className="max-w-xl whitespace-normal">{item.value}</TableCell>
                        <TableCell className="text-right">
                          {auth.can("translation:read") && (
                            <Button nativeButton={false} size="icon-sm" variant="ghost" aria-label={`Edit ${item.key}`} render={<Link to="/translations/$locale/$key" params={{ locale: selectedLocale, key: item.key }} />}><PencilSimpleIcon /></Button>
                          )}
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
                {entries.length === 0 && <p className="px-4 py-8 text-sm text-muted-foreground">No matching translations.</p>}
              </div>
              <div className="mt-4 flex justify-center">
                <Button variant="outline" disabled={!translation.listQuery.hasNextPage || translation.listQuery.isFetchingNextPage} onClick={() => void translation.listQuery.fetchNextPage()}>
                  {translation.listQuery.isFetchingNextPage ? "Loading…" : "Load more"} <CaretRightIcon />
                </Button>
              </div>
            </>
          )}
        </CardContent>
      </Card>
    </div>
  )
}