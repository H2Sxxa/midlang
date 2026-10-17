import {
  ArrowCounterClockwiseIcon,
  CheckCircleIcon,
  EyeSlashIcon,
} from "@phosphor-icons/react"
import { createFileRoute } from "@tanstack/react-router"
import { CursorPagination } from "@/components/cursor-pagination"
import { DataTableCard, type TableState } from "@/components/data-table-card"
import { FilterSelect } from "@/components/filter-select"
import { OrderButton } from "@/components/order-button"
import { SearchInput } from "@/components/search-input"
import { StateBadge } from "@/components/state-badge"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { useAuth } from "@/hooks/use-auth"
import { useSearchQuery } from "@/hooks/use-search-query"
import { useToast } from "@/hooks/use-toast"
import {
  ISSUE_KINDS,
  ISSUE_STATES,
  useIssues,
  type Issue,
  type IssueKind,
  type IssueSort,
  type IssueState,
} from "@/hooks/use-issues"
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

const STATE_VALUES: ReadonlyArray<IssueState | ""> = ["", ...ISSUE_STATES]
const STATE_LABELS: Readonly<Record<IssueState | "", string>> = {
  "": "All states",
  open: "Open",
  closed: "Closed",
  ignored: "Ignored",
}
const STATE_OPTIONS: ReadonlyArray<SelectOption> = STATE_VALUES.map((value) => ({
  value,
  label: STATE_LABELS[value],
}))

const KIND_VALUES: ReadonlyArray<IssueKind | ""> = ["", ...ISSUE_KINDS]
const KIND_LABELS: Readonly<Record<string, string>> = {
  "": "All types",
  missing_translation: "Missing translation",
  missing_locale: "Missing locale",
}
const KIND_OPTIONS: ReadonlyArray<SelectOption> = KIND_VALUES.map((value) => ({
  value,
  label: KIND_LABELS[value],
}))

const SORT_VALUES: ReadonlyArray<IssueSort> = ["last_seen", "created_at", "count"]
const SORT_LABELS: Readonly<Record<IssueSort, string>> = {
  last_seen: "Last seen",
  created_at: "Created",
  count: "Count",
}
const SORT_OPTIONS: ReadonlyArray<SelectOption> = SORT_VALUES.map((value) => ({
  value,
  label: SORT_LABELS[value],
}))

type IssuesSearch = {
  readonly q?: string
  readonly state?: IssueState | ""
  readonly kind?: IssueKind | ""
  readonly sort?: IssueSort
  readonly order?: SortOrder
  readonly cursor?: string
}

/** Reads the list params, filling in whatever the URL leaves out. */
function resolveIssuesSearch(search: Record<string, unknown>): Required<IssuesSearch> {
  return {
    q: parseText(search.q),
    state: parseChoice(search.state, STATE_VALUES, ""),
    kind: parseChoice(search.kind, KIND_VALUES, ""),
    sort: parseChoice(search.sort, SORT_VALUES, "last_seen"),
    order: parseChoice(search.order, SORT_ORDERS, "desc"),
    cursor: parseText(search.cursor),
  }
}

export const Route = createFileRoute("/_app/issues")({
  // The route's search type stays optional, so a link may omit every param.
  validateSearch: (search: Record<string, unknown>): IssuesSearch => resolveIssuesSearch(search),
  component: IssuesPage,
})

function IssuesPage() {
  const auth = useAuth()
  const toast = useToast()
  const navigate = Route.useNavigate()
  const search = resolveIssuesSearch(Route.useSearch())
  const canManage = auth.can("diagnostic:manage")

  const changeFilters = (patch: Partial<IssuesSearch>) => {
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

  const issues = useIssues({
    keyword: search.q.trim(),
    state: search.state,
    kind: search.kind,
    sort: search.sort,
    order: search.order,
    cursor: search.cursor,
    onCursorChange: changeCursor,
  })
  const list = issues.list

  const changeState = (issue: Issue, next: IssueState) => {
    issues.setStateMutation.mutate(
      { id: issue.id, state: next },
      {
        onSuccess: () =>
          toast.success({
            message: `Issue marked ${next}`,
            description: issue.summary,
          }),
        onError: (error) =>
          toast.error({ message: "Unable to update issue", description: error.message }),
      },
    )
  }

  if (!auth.can("diagnostic:read")) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Issues unavailable</CardTitle>
          <CardDescription>You need the diagnostic:read permission to view issues.</CardDescription>
        </CardHeader>
      </Card>
    )
  }

  const tableState: TableState = list.isError
    ? { kind: "error", message: list.error?.message ?? "Unable to load issues." }
    : list.isLoading
      ? { kind: "loading", message: "Loading issues…" }
      : list.items.length === 0
        ? { kind: "empty", message: "No issues match the current filters." }
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
        <h1 className="text-2xl font-semibold tracking-tight">Issues</h1>
        <p className="mt-1 text-muted-foreground">
          Find missing translation keys reported by your applications.
        </p>
      </div>

      <Card>
        <CardContent className="flex flex-col gap-3 pt-6 md:flex-row md:items-end">
          <SearchInput
            id="keyword"
            label="Search issues"
            placeholder="Search summary, payload or type"
            value={keywordQuery.draft}
            onChange={keywordQuery.setDraft}
          />
          <FilterSelect
            id="state"
            label="State"
            placeholder="All states"
            value={search.state}
            options={STATE_OPTIONS}
            onChange={(value) => changeFilters({ state: parseChoice(value, STATE_VALUES, "") })}
          />
          <FilterSelect
            id="kind"
            label="Type"
            placeholder="All types"
            value={search.kind}
            options={KIND_OPTIONS}
            onChange={(value) => changeFilters({ kind: parseChoice(value, KIND_VALUES, "") })}
          />
          <FilterSelect
            id="sort"
            label="Sort"
            placeholder="Last seen"
            value={search.sort}
            options={SORT_OPTIONS}
            onChange={(value) =>
              changeFilters({ sort: parseChoice(value, SORT_VALUES, "last_seen") })
            }
          />
          <OrderButton
            id="order"
            order={search.order}
            onChange={(order) => changeFilters({ order })}
          />
        </CardContent>
      </Card>

      <DataTableCard
        title="Reported issues"
        description={
          search.q
            ? `Results matching “${search.q}”.`
            : "Missing keys reported by your applications."
        }
        state={tableState}
        footer={pager}
      >
        <div className="rounded-md border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Type</TableHead>
                <TableHead>Details</TableHead>
                <TableHead className="text-right">Count</TableHead>
                <TableHead>Last seen</TableHead>
                <TableHead>State</TableHead>
                {canManage && <TableHead className="text-right">Actions</TableHead>}
              </TableRow>
            </TableHeader>
            <TableBody>
              {list.items.map((issue) => (
                <TableRow key={issue.id}>
                  <TableCell>
                    <Badge variant="secondary">{issue.kind}</Badge>
                  </TableCell>
                  <TableCell>{issue.summary}</TableCell>
                  <TableCell className="text-right tabular-nums">{issue.count}</TableCell>
                  <TableCell className="text-muted-foreground">
                    {formatTimestamp(issue.last_seen)}
                  </TableCell>
                  <TableCell>
                    <StateBadge state={issue.state} />
                  </TableCell>
                  {canManage && (
                    <TableCell className="text-right">
                      <div className="flex justify-end gap-1">
                        {issue.state !== "open" && (
                          <Button
                            size="icon-sm"
                            variant="ghost"
                            aria-label={`Reopen ${issue.summary}`}
                            disabled={issues.setStateMutation.isPending}
                            onClick={() => changeState(issue, "open")}
                          >
                            <ArrowCounterClockwiseIcon />
                          </Button>
                        )}
                        {issue.state !== "closed" && (
                          <Button
                            size="icon-sm"
                            variant="ghost"
                            aria-label={`Close ${issue.summary}`}
                            disabled={issues.setStateMutation.isPending}
                            onClick={() => changeState(issue, "closed")}
                          >
                            <CheckCircleIcon />
                          </Button>
                        )}
                        {issue.state !== "ignored" && (
                          <Button
                            size="icon-sm"
                            variant="ghost"
                            aria-label={`Ignore ${issue.summary}`}
                            disabled={issues.setStateMutation.isPending}
                            onClick={() => changeState(issue, "ignored")}
                          >
                            <EyeSlashIcon />
                          </Button>
                        )}
                      </div>
                    </TableCell>
                  )}
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      </DataTableCard>
    </div>
  )
}
