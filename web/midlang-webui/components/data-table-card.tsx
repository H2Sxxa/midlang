import type { ReactNode } from "react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"

/** What the table area shows: the rows, or the single thing standing in for them. */
export type TableState =
  | { readonly kind: "loading"; readonly message: string }
  | { readonly kind: "error"; readonly message: string }
  | { readonly kind: "empty"; readonly message: string }
  | { readonly kind: "ready" }

type DataTableCardProps = {
  readonly title: string
  readonly description: string
  readonly state: TableState
  readonly footer: ReactNode
  readonly children: ReactNode
}

/**
 * Card around a list table, so every list renders its loading, empty and error
 * states the same way. The footer stays visible in every state, so a reader
 * whose page failed or emptied out can still step back along the trail.
 */
export function DataTableCard({ title, description, state, footer, children }: DataTableCardProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      <CardContent>
        {state.kind === "loading" && (
          <p className="py-8 text-sm text-muted-foreground">{state.message}</p>
        )}
        {state.kind === "error" && (
          <p className="py-8 text-sm text-destructive">{state.message}</p>
        )}
        {state.kind === "empty" && (
          <p className="py-8 text-sm text-muted-foreground">{state.message}</p>
        )}
        {state.kind === "ready" && children}
        {footer}
      </CardContent>
    </Card>
  )
}
