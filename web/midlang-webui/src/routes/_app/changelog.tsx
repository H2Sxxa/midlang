import {
  ArrowCounterClockwiseIcon,
  MagnifyingGlassIcon,
} from "@phosphor-icons/react"
import { createFileRoute } from "@tanstack/react-router"
import { useState, type SubmitEvent } from "react"
import { ConfirmDialog } from "@/components/access/confirm-dialog"
import { FilterSelect } from "@/components/filter-select"
import { LoadMore } from "@/components/load-more"
import { OrderButton } from "@/components/order-button"
import { StateBadge } from "@/components/state-badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { useAuth } from "@/hooks/use-auth"
import { useChangelog, type Change, type ChangeState } from "@/hooks/use-changelog"
import { useToast } from "@/hooks/use-toast"
import { formatTimestamp } from "@/lib/format"
import type { SelectOption, SortOrder } from "@/lib/query"

export const Route = createFileRoute("/_app/changelog")({
  component: ChangelogPage,
})

const STATE_OPTIONS: ReadonlyArray<SelectOption> = [
  { value: "", label: "All changes" },
  { value: "created", label: "Created" },
  { value: "updated", label: "Updated" },
  { value: "deleted", label: "Deleted" },
]

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
  const [keyword, setKeyword] = useState("")
  const [appliedKeyword, setAppliedKeyword] = useState("")
  const [locale, setLocale] = useState("")
  const [state, setState] = useState<ChangeState | "">("")
  const [order, setOrder] = useState<SortOrder>("desc")
  const [pendingRollback, setPendingRollback] = useState<Change | null>(null)
  const changelog = useChangelog({
    keyword: appliedKeyword,
    locale: locale.trim(),
    state,
    order,
  })
  const items = changelog.listQuery.data?.pages.flatMap((page) => page.items) ?? []
  const canRollback = auth.can("diagnostic:read") && auth.can("translation:write")

  const handleSearch = (event: SubmitEvent) => {
    event.preventDefault()
    setAppliedKeyword(keyword.trim())
  }

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
          <form className="flex flex-1 gap-2" onSubmit={handleSearch}>
            <div className="relative flex-1">
              <MagnifyingGlassIcon className="absolute top-2 left-2.5 size-4 text-muted-foreground" />
              <Input
                className="pl-8"
                placeholder="Search key or value"
                value={keyword}
                onChange={(event) => setKeyword(event.target.value)}
              />
            </div>
            <Button type="submit" variant="outline">
              Search
            </Button>
          </form>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="locale">Locale</Label>
            <Input
              id="locale"
              className="w-44"
              placeholder="All locales"
              value={locale}
              onChange={(event) => setLocale(event.target.value)}
            />
          </div>
          <FilterSelect
            id="state"
            label="Change"
            placeholder="All changes"
            value={state}
            options={STATE_OPTIONS}
            onChange={(value) => setState(value as ChangeState | "")}
          />
          <OrderButton id="order" order={order} onChange={setOrder} />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Recorded changes</CardTitle>
          <CardDescription>
            {appliedKeyword
              ? `Results matching “${appliedKeyword}”.`
              : "Changes recorded by translation writes and deletes."}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {changelog.listQuery.isError ? (
            <p className="py-8 text-sm text-destructive">{changelog.listQuery.error.message}</p>
          ) : changelog.listQuery.isLoading ? (
            <p className="py-8 text-sm text-muted-foreground">Loading changes…</p>
          ) : items.length === 0 ? (
            <p className="py-8 text-sm text-muted-foreground">
              No changes match the current filters.
            </p>
          ) : (
            <>
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
                    {items.map((change) => (
                      <TableRow key={change.id}>
                        <TableCell className="text-muted-foreground tabular-nums">
                          #{change.id}
                        </TableCell>
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
              <LoadMore
                hasNextPage={changelog.listQuery.hasNextPage}
                isFetching={changelog.listQuery.isFetchingNextPage}
                onLoadMore={() => void changelog.listQuery.fetchNextPage()}
              />
            </>
          )}
        </CardContent>
      </Card>

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
