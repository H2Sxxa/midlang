import {
  ArrowCounterClockwiseIcon,
  CheckCircleIcon,
  EyeSlashIcon,
  MagnifyingGlassIcon,
} from "@phosphor-icons/react"
import { createFileRoute } from "@tanstack/react-router"
import { useState, type SubmitEvent } from "react"
import { FilterSelect } from "@/components/filter-select"
import { LoadMore } from "@/components/load-more"
import { OrderButton } from "@/components/order-button"
import { StateBadge } from "@/components/state-badge"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { useAuth } from "@/hooks/use-auth"
import {
  useIssues,
  type Issue,
  type IssueKind,
  type IssueSort,
  type IssueState,
} from "@/hooks/use-issues"
import { useToast } from "@/hooks/use-toast"
import { formatTimestamp } from "@/lib/format"
import type { SelectOption, SortOrder } from "@/lib/query"

export const Route = createFileRoute("/_app/issues")({
  component: IssuesPage,
})

const STATE_OPTIONS: ReadonlyArray<SelectOption> = [
  { value: "", label: "All states" },
  { value: "open", label: "Open" },
  { value: "closed", label: "Closed" },
  { value: "ignored", label: "Ignored" },
]

const KIND_OPTIONS: ReadonlyArray<SelectOption> = [
  { value: "", label: "All types" },
  { value: "missing_translation", label: "Missing translation" },
  { value: "missing_locale", label: "Missing locale" },
]

const SORT_OPTIONS: ReadonlyArray<SelectOption> = [
  { value: "last_seen", label: "Last seen" },
  { value: "created_at", label: "Created" },
  { value: "count", label: "Count" },
]

function IssuesPage() {
  const auth = useAuth()
  const toast = useToast()
  const [keyword, setKeyword] = useState("")
  const [appliedKeyword, setAppliedKeyword] = useState("")
  const [state, setState] = useState<IssueState | "">("")
  const [kind, setKind] = useState<IssueKind | "">("")
  const [sort, setSort] = useState<IssueSort>("last_seen")
  const [order, setOrder] = useState<SortOrder>("desc")
  const issues = useIssues({ keyword: appliedKeyword, state, kind, sort, order })
  const items = issues.listQuery.data?.pages.flatMap((page) => page.items) ?? []
  const canManage = auth.can("diagnostic:manage")

  const handleSearch = (event: SubmitEvent) => {
    event.preventDefault()
    setAppliedKeyword(keyword.trim())
  }

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

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Issues</h1>
        <p className="mt-1 text-muted-foreground">Find missing translation keys reported by your applications.</p>
      </div>

      <Card>
        <CardContent className="flex flex-col gap-3 pt-6 md:flex-row md:items-end">
          <form className="flex flex-1 gap-2" onSubmit={handleSearch}>
            <div className="relative flex-1">
              <MagnifyingGlassIcon className="absolute top-2 left-2.5 size-4 text-muted-foreground" />
              <Input
                className="pl-8"
                placeholder="Search summary, payload or type"
                value={keyword}
                onChange={(event) => setKeyword(event.target.value)}
              />
            </div>
            <Button type="submit" variant="outline">
              Search
            </Button>
          </form>
          <FilterSelect
            id="state"
            label="State"
            placeholder="All states"
            value={state}
            options={STATE_OPTIONS}
            onChange={(value) => setState(value as IssueState | "")}
          />
          <FilterSelect
            id="kind"
            label="Type"
            placeholder="All types"
            value={kind}
            options={KIND_OPTIONS}
            onChange={(value) => setKind(value)}
          />
          <FilterSelect
            id="sort"
            label="Sort"
            placeholder="Last seen"
            value={sort}
            options={SORT_OPTIONS}
            onChange={(value) => setSort(value as IssueSort)}
          />
          <OrderButton id="order" order={order} onChange={setOrder} />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Reported issues</CardTitle>
          <CardDescription>
            {appliedKeyword ? `Results matching \u201c${appliedKeyword}\u201d.` : "Missing keys reported by your applications."}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {issues.listQuery.isError ? (
            <p className="py-8 text-sm text-destructive">{issues.listQuery.error.message}</p>
          ) : issues.listQuery.isLoading ? (
            <p className="py-8 text-sm text-muted-foreground">Loading issues…</p>
          ) : items.length === 0 ? (
            <p className="py-8 text-sm text-muted-foreground">No issues match the current filters.</p>
          ) : (
            <>
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
                    {items.map((issue) => (
                      <TableRow key={issue.id}>
                        <TableCell>
                          <Badge variant="secondary">{issue.kind}</Badge>
                        </TableCell>
                        <TableCell>{issue.summary}</TableCell>
                        <TableCell className="text-right tabular-nums">{issue.count}</TableCell>
                        <TableCell className="text-muted-foreground">{formatTimestamp(issue.last_seen)}</TableCell>
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
              <LoadMore
                hasNextPage={issues.listQuery.hasNextPage}
                isFetching={issues.listQuery.isFetchingNextPage}
                onLoadMore={() => void issues.listQuery.fetchNextPage()}
              />
            </>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
