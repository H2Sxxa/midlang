import { ChartBarIcon } from "@phosphor-icons/react"
import { createFileRoute } from "@tanstack/react-router"
import { useState } from "react"
import { FilterSelect } from "@/components/filter-select"
import { LoadMore } from "@/components/load-more"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { useAuth } from "@/hooks/use-auth"
import {
  useCoverageLocales,
  useCoverageReport,
  useMissingKeys,
  type CoverageReport,
  type LocaleCoverage,
} from "@/hooks/use-coverage"

export const Route = createFileRoute("/_app/coverage")({
  component: CoveragePage,
})

/** Formats a 0..1 ratio as a whole-number percentage. */
function formatRatio(ratio: number): string {
  return `${Math.round(ratio * 100)}%`
}

/** Horizontal bar showing how much of the reference locale a locale translates. */
function CoverageBar({ ratio }: { readonly ratio: number }) {
  const percent = Math.min(100, Math.max(0, ratio * 100))
  return (
    <div className="flex items-center gap-2">
      <div className="h-2 w-32 overflow-hidden rounded-full bg-muted">
        <div className="h-full rounded-full bg-primary" style={{ width: `${percent}%` }} />
      </div>
      <span className="tabular-nums text-muted-foreground">{formatRatio(ratio)}</span>
    </div>
  )
}

/** Paged list of the reference keys a locale does not translate. */
function MissingKeysPanel({
  reference,
  locale,
  onClose,
}: {
  readonly reference: string
  readonly locale: string
  readonly onClose: () => void
}) {
  const missing = useMissingKeys(reference, locale)
  const keys = missing.data?.pages.flatMap((page) => page.items) ?? []

  return (
    <Card>
      <CardHeader className="flex flex-row items-start justify-between gap-4">
        <div className="flex flex-col gap-1">
          <CardTitle>Missing keys in {locale}</CardTitle>
          <CardDescription>
            Keys that “{reference}” translates and “{locale}” does not.
          </CardDescription>
        </div>
        <Button variant="ghost" size="sm" onClick={onClose}>
          Close
        </Button>
      </CardHeader>
      <CardContent>
        {missing.isError ? (
          <p className="py-4 text-sm text-destructive">{missing.error.message}</p>
        ) : missing.isLoading ? (
          <p className="py-4 text-sm text-muted-foreground">Loading missing keys…</p>
        ) : keys.length === 0 ? (
          <p className="py-4 text-sm text-muted-foreground">
            {locale} translates every key the reference defines.
          </p>
        ) : (
          <>
            <ul className="grid gap-1 md:grid-cols-2">
              {keys.map((key) => (
                <li
                  key={key}
                  className="truncate rounded-sm bg-muted/50 px-2 py-1 font-mono text-xs"
                  title={key}
                >
                  {key}
                </li>
              ))}
            </ul>
            <LoadMore
              hasNextPage={missing.hasNextPage}
              isFetching={missing.isFetchingNextPage}
              onLoadMore={() => void missing.fetchNextPage()}
            />
          </>
        )}
      </CardContent>
    </Card>
  )
}

/** Per-locale rows, least covered first so gaps surface at the top. */
function rowsOf(report: CoverageReport): ReadonlyArray<readonly [string, LocaleCoverage]> {
  return Object.entries(report.per_locale).sort(([leftLocale, left], [rightLocale, right]) => {
    if (left.ratio !== right.ratio) {
      return left.ratio - right.ratio
    }
    return leftLocale.localeCompare(rightLocale)
  })
}

function CoveragePage() {
  const auth = useAuth()
  const [reference, setReference] = useState("")
  const [selectedLocale, setSelectedLocale] = useState<string | null>(null)
  const { localesQuery, locales } = useCoverageLocales()
  const activeReference = reference === "" ? (locales[0] ?? "") : reference
  const reportQuery = useCoverageReport(activeReference)

  if (!auth.can("diagnostic:read")) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Coverage unavailable</CardTitle>
          <CardDescription>
            You need the diagnostic:read permission to view coverage.
          </CardDescription>
        </CardHeader>
      </Card>
    )
  }

  const report = reportQuery.data ?? null
  const rows = report === null ? [] : rowsOf(report)

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Coverage</h1>
        <p className="mt-1 text-muted-foreground">
          Track translation coverage by locale against a reference locale.
        </p>
      </div>

      {localesQuery.isError ? (
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-destructive">{localesQuery.error.message}</p>
          </CardContent>
        </Card>
      ) : localesQuery.isLoading ? (
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">Loading locales…</p>
          </CardContent>
        </Card>
      ) : locales.length === 0 ? (
        <Card>
          <CardHeader>
            <CardTitle>No locales yet</CardTitle>
            <CardDescription>
              Add at least one translation before coverage can be measured.
            </CardDescription>
          </CardHeader>
        </Card>
      ) : (
        <>
          <Card>
            <CardContent className="flex flex-col gap-3 pt-6 md:flex-row md:items-end">
              <FilterSelect
                id="reference"
                label="Reference locale"
                placeholder="Select a locale"
                value={activeReference}
                options={locales.map((locale) => ({ value: locale, label: locale }))}
                onChange={(value) => {
                  setReference(value)
                  setSelectedLocale(null)
                }}
              />
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <ChartBarIcon className="size-4 text-muted-foreground" />
                Coverage against {activeReference}
              </CardTitle>
              <CardDescription>
                {report === null
                  ? `How much every other locale translates of ${activeReference}.`
                  : `${report.reference_keys} keys defined by ${report.reference} across ${
                      Object.keys(report.per_locale).length
                    } locales · ${formatRatio(report.overall)} overall.`}
              </CardDescription>
            </CardHeader>
            <CardContent>
              {reportQuery.isError ? (
                <p className="py-8 text-sm text-destructive">{reportQuery.error.message}</p>
              ) : reportQuery.isLoading ? (
                <p className="py-8 text-sm text-muted-foreground">Measuring coverage…</p>
              ) : rows.length === 0 ? (
                <p className="py-8 text-sm text-muted-foreground">
                  This is the only locale in the store, so there is nothing to measure yet.
                </p>
              ) : (
                <div className="rounded-md border">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Locale</TableHead>
                        <TableHead className="text-right">Translated</TableHead>
                        <TableHead className="text-right">Missing</TableHead>
                        <TableHead>Coverage</TableHead>
                        <TableHead className="text-right">Actions</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {rows.map(([locale, coverage]) => (
                        <TableRow key={locale}>
                          <TableCell className="font-medium">{locale}</TableCell>
                          <TableCell className="text-right tabular-nums">
                            {coverage.present}
                          </TableCell>
                          <TableCell className="text-right tabular-nums">
                            {coverage.missing}
                          </TableCell>
                          <TableCell>
                            <CoverageBar ratio={coverage.ratio} />
                          </TableCell>
                          <TableCell className="text-right">
                            <Button
                              variant="outline"
                              size="sm"
                              disabled={coverage.missing === 0}
                              onClick={() => setSelectedLocale(locale)}
                            >
                              Missing keys
                            </Button>
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </div>
              )}
            </CardContent>
          </Card>

          {selectedLocale !== null && (
            <MissingKeysPanel
              reference={activeReference}
              locale={selectedLocale}
              onClose={() => setSelectedLocale(null)}
            />
          )}
        </>
      )}
    </div>
  )
}
