import { ChartBarIcon } from "@phosphor-icons/react"
import { createFileRoute, Link } from "@tanstack/react-router"
import { FilterSelect } from "@/components/filter-select"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { useAuth } from "@/hooks/use-auth"
import {
  useCoverageLocales,
  useCoverageReport,
  type CoverageReport,
  type LocaleCoverage,
} from "@/hooks/use-coverage"
import { parseText } from "@/lib/query"

/** Reference locale the report measures against; the first locale when absent. */
type CoverageSearch = {
  readonly reference?: string
}

/** Reads the page params, filling in whatever the URL leaves out. */
function resolveCoverageSearch(search: Record<string, unknown>): Required<CoverageSearch> {
  return { reference: parseText(search.reference) }
}

export const Route = createFileRoute("/_app/coverage/")({
  // The route's search type stays optional, so a link may omit every param.
  validateSearch: (search: Record<string, unknown>): CoverageSearch => resolveCoverageSearch(search),
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
  const navigate = Route.useNavigate()
  const search = resolveCoverageSearch(Route.useSearch())
  const { localesQuery, locales } = useCoverageLocales()
  const activeReference = search.reference === "" ? (locales[0] ?? "") : search.reference
  const reportQuery = useCoverageReport(activeReference)

  const changeReference = (reference: string) => {
    void navigate({ search: (prev) => ({ ...prev, reference }) })
  }

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
                onChange={changeReference}
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
                            {coverage.missing === 0 ? (
                              <Button variant="outline" size="sm" disabled>
                                Missing keys
                              </Button>
                            ) : (
                              <Button
                                nativeButton={false}
                                variant="outline"
                                size="sm"
                                render={
                                  <Link
                                    to="/coverage/$reference/$locale"
                                    params={{ reference: activeReference, locale }}
                                  />
                                }
                              >
                                Missing keys
                              </Button>
                            )}
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </div>
              )}
            </CardContent>
          </Card>
        </>
      )}
    </div>
  )
}
