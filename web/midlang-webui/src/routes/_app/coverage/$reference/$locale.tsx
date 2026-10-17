import { ArrowLeftIcon } from "@phosphor-icons/react"
import { createFileRoute, Link } from "@tanstack/react-router"
import { CursorPagination } from "@/components/cursor-pagination"
import { DataTableCard, type TableState } from "@/components/data-table-card"
import { Button } from "@/components/ui/button"
import { Card, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { useAuth } from "@/hooks/use-auth"
import { useMissingKeys } from "@/hooks/use-coverage"
import { FIRST_PAGE, parseText } from "@/lib/query"

/** Cursor of the missing-key page to read; absent on the first page. */
type MissingKeysSearch = {
  readonly cursor?: string
}

/** Reads the page params, filling in whatever the URL leaves out. */
function resolveMissingKeysSearch(search: Record<string, unknown>): Required<MissingKeysSearch> {
  return { cursor: parseText(search.cursor) }
}

export const Route = createFileRoute("/_app/coverage/$reference/$locale")({
  // The route's search type stays optional, so a link may omit every param.
  validateSearch: (search: Record<string, unknown>): MissingKeysSearch =>
    resolveMissingKeysSearch(search),
  component: MissingKeysPage,
})

/** Every reference key `locale` does not translate, in ascending key order. */
function MissingKeysPage() {
  const auth = useAuth()
  const { reference, locale } = Route.useParams()
  const navigate = Route.useNavigate()
  const search = resolveMissingKeysSearch(Route.useSearch())

  const missing = useMissingKeys({
    reference,
    locale,
    cursor: search.cursor,
    onCursorChange: (cursor) => {
      void navigate({
        search: (prev) => ({ ...prev, cursor: cursor === FIRST_PAGE ? undefined : cursor }),
      })
    },
  })

  if (!auth.can("diagnostic:read")) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Missing keys unavailable</CardTitle>
          <CardDescription>
            You need the diagnostic:read permission to view missing keys.
          </CardDescription>
        </CardHeader>
      </Card>
    )
  }

  const tableState: TableState = missing.isError
    ? { kind: "error", message: missing.error?.message ?? "Unable to load missing keys." }
    : missing.isLoading
      ? { kind: "loading", message: "Loading missing keys…" }
      : missing.items.length === 0
        ? {
            kind: "empty",
            message: missing.isFirst
              ? `${locale} translates every key the reference defines.`
              : "This page holds no keys any more.",
          }
        : { kind: "ready" }

  return (
    <div className="space-y-6">
      <Button
        nativeButton={false}
        variant="ghost"
        render={<Link to="/coverage" search={{ reference }} />}
      >
        <ArrowLeftIcon /> Back to coverage
      </Button>
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Missing keys in {locale}</h1>
        <p className="mt-1 text-muted-foreground">
          Keys that “{reference}” translates and “{locale}” does not.
        </p>
      </div>

      <DataTableCard
        title="Untranslated keys"
        description={`Compared against “${reference}”.`}
        state={tableState}
        footer={
          <CursorPagination
            isFirst={missing.isFirst}
            hasPrev={missing.hasPrev}
            hasNext={missing.hasNext}
            isFetching={missing.isFetching}
            onFirst={missing.goFirst}
            onPrev={missing.goPrev}
            onNext={missing.goNext}
          />
        }
      >
        <ul className="grid gap-1 md:grid-cols-2">
          {missing.items.map((key) => (
            <li
              key={key}
              className="truncate rounded-sm bg-muted/50 px-2 py-1 font-mono text-xs"
              title={key}
            >
              {key}
            </li>
          ))}
        </ul>
      </DataTableCard>
    </div>
  )
}
