import { ArrowCounterClockwiseIcon } from "@phosphor-icons/react"
import { createFileRoute } from "@tanstack/react-router"
import { useState } from "react"
import { ConfirmDialog } from "@/components/access/confirm-dialog"
import { CursorPagination } from "@/components/cursor-pagination"
import { DataTableCard, type TableState } from "@/components/data-table-card"
import { FilterSelect } from "@/components/filter-select"
import { OrderButton } from "@/components/order-button"
import { SearchInput } from "@/components/search-input"
import { StateBadge } from "@/components/state-badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { useAuth } from "@/hooks/use-auth"
import { useSearchQuery } from "@/hooks/use-search-query"
import { useToast } from "@/hooks/use-toast"
import { CHANGE_STATES, useChangelog, type Change, type ChangeState } from "@/hooks/use-changelog"
import { formatTimestamp } from "@/lib/format"
import {
  FIRST_PAGE,
  SORT_ORDERS,
  parseChoice,
  parseText,
  type SelectOption,
  type SortOrder,
} from "@/lib/query"

/** How long typing pauses before a text filter applies. */
const SEARCH_WAIT_MS = 300

const STATE_VALUES: ReadonlyArray<ChangeState | ""> = ["", ...CHANGE_STATES]
const STATE_LABELS: Readonly<Record<ChangeState | "", string>> = {
  "": "All changes",
  created: "Created",
  updated: "Updated",
  deleted: "Deleted",
}
const STATE_OPTIONS: ReadonlyArray<SelectOption> = STATE_VALUES.map((value) => ({
  value,
  label: STATE_LABELS[value],
}))

type ChangelogSearch = {
  readonly q?: string
  readonly locale?: string
  readonly state?: ChangeState | ""
  readonly order?: SortOrder
  readonly cursor?: string
}

/** Reads the list params, filling in whatever the URL leaves out. */
function resolveChangelogSearch(search: Record<string, unknown>): Required<ChangelogSearch> {
  return {
    q: parseText(search.q),
    locale: parseText(search.locale),
    state: parseChoice(search.state, STATE_VALUES, ""),
    order: parseChoice(search.order, SORT_ORDERS, "desc"),
    cursor: parseText(search.cursor),
  }
}

export const Route = createFileRoute("/_app/changelog")({
  // The route's search type stays optional, so a link may omit every param.
  validateSearch: (search: Record<string, unknown>): ChangelogSearch =>
    resolveChangelogSearch(search),
  component: ChangelogPage,
})

/** Renders one side of a change, or a dash when that side has no value. */
function ValueCell({ value }: { readonly value: string | null | undefined }) {
  if (value === null || value === undefined) {
    return <span className="text-muted-foreground">—</span>
  }
  return (
    <span className="block max-w-xs truncate" title={value}>
      {value}
    </span>
  )
}

function rollbackDescription(change: Change): string {
  if (change.previous_value === null || change.previous_value === undefined) {
    return `This deletes “${change.key}” in ${change.locale}, restoring the state it had before change #${change.id}.`
  }
  return `This restores “${change.key}” in ${change.locale} to “${change.previous_value}”, undoing change #${change.id}.`
}

function ChangelogPage() {
  const auth = useAuth()
  const toast = useToast()
  const navigate = Route.useNavigate()
  const search = resolveChangelogSearch(Route.useSearch())
  const [pendingRollback, setPendingRollback] = useState<Change | null>(null)

  const changeFilters = (patch: Partial<ChangelogSearch>) => {
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

  const changelog = useChangelog({
    keyword: search.q.trim(),
    locale: search.locale.trim(),
    state: search.state,
    order: search.order,
    cursor: search.cursor,
    onCursorChange: changeCursor,
  })
  const list = changelog.list
  const canRollback = auth.can("diagnostic:read") && auth.can("translation:write")

  const confirmRollback = () => {
    const change = pendingRollback
    if (change === null) {
      return
    }
    changelog.rollbackMutation.mutate(change.id, {
      onSuccess: () => {
        setPendingRollback(null)
        toast.success({
          message: `Rolled back change #${change.id}`,
          description: `${change.key} in ${change.locale}`,
        })
      },
      onError: (error) =>
        toast.error({ message: "Unable to roll back change", description: error.message }),
    })
  }

  if (!auth.can("diagnostic:read")) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Changelog unavailable</CardTitle>
          <CardDescription>
            You need the diagnostic:read permission to view the changelog.
          </CardDescription>
        </CardHeader>
      </Card>
    )
  }

  const tableState: TableState = list.isError
    ? { kind: "error", message: list.error?.message ?? "Unable to load changes." }
    : list.isLoading
      ? { kind: "loading", message: "Loading changes…" }
      : list.items.length === 0
        ? { kind: "empty", message: "No changes match the current filters." }
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
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Changelog</h1>
        <p className="mt-1 text-muted-foreground">
          Every recorded translation change, newest first. Rolling one back restores the value it
          replaced.
        </p>
      </div>

      <Card>
        <CardContent className="flex flex-col gap-3 pt-6 md:flex-row md:items-end">
          <SearchInput
            id="keyword"
            label="Search changes"
            placeholder="Search key or value"
            value={keywordQuery.draft}
            onChange={keywordQuery.setDraft}
          />
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="locale">Locale</Label>
            <Input
              id="locale"
              className="w-44"
              placeholder="All locales"
              value={localeQuery.draft}
              onChange={(event) => localeQuery.setDraft(event.target.value)}
            />
          </div>
          <FilterSelect
            id="state"
            label="Change"
            placeholder="All changes"
            value={search.state}
            options={STATE_OPTIONS}
            onChange={(value) => changeFilters({ state: parseChoice(value, STATE_VALUES, "") })}
          />
          <OrderButton
            id="order"
            order={search.order}
            onChange={(order) => changeFilters({ order })}
          />
        </CardContent>
      </Card>

      <DataTableCard
        title="Recorded changes"
        description={
          search.q
            ? `Results matching “${search.q}”.`
            : "Changes recorded by translation writes and deletes."
        }
        state={tableState}
        footer={pager}
      >
        <div className="rounded-md border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Entry</TableHead>
                <TableHead>When</TableHead>
                <TableHead>Locale</TableHead>
                <TableHead>Key</TableHead>
                <TableHead>Change</TableHead>
                <TableHead>Before</TableHead>
                <TableHead>After</TableHead>
                {canRollback && <TableHead className="text-right">Actions</TableHead>}
              </TableRow>
            </TableHeader>
            <TableBody>
              {list.items.map((change) => (
                <TableRow key={change.id}>
                  <TableCell className="text-muted-foreground tabular-nums">#{change.id}</TableCell>
                  <TableCell className="text-muted-foreground whitespace-nowrap">
                    {formatTimestamp(change.created_at)}
                  </TableCell>
                  <TableCell>{change.locale}</TableCell>
                  <TableCell className="font-mono text-xs">{change.key}</TableCell>
                  <TableCell>
                    <StateBadge state={change.state} />
                  </TableCell>
                  <TableCell>
                    <ValueCell value={change.previous_value} />
                  </TableCell>
                  <TableCell>
                    <ValueCell value={change.new_value} />
                  </TableCell>
                  {canRollback && (
                    <TableCell className="text-right">
                      <Button
                        size="icon-sm"
                        variant="ghost"
                        aria-label={`Roll back change ${change.id}`}
                        disabled={changelog.rollbackMutation.isPending}
                        onClick={() => setPendingRollback(change)}
                      >
                        <ArrowCounterClockwiseIcon />
                      </Button>
                    </TableCell>
                  )}
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      </DataTableCard>

      <ConfirmDialog
        open={pendingRollback !== null}
        onOpenChange={(open) => {
          if (!open) {
            setPendingRollback(null)
          }
        }}
        title="Roll back this change?"
        description={pendingRollback === null ? "" : rollbackDescription(pendingRollback)}
        confirmLabel="Roll back"
        pending={changelog.rollbackMutation.isPending}
        destructive
        onConfirm={confirmRollback}
      />
    </div>
  )
}
