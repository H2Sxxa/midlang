import { SortAscendingIcon, SortDescendingIcon } from "@phosphor-icons/react"
import { Button } from "@/components/ui/button"
import { Label } from "@/components/ui/label"
import type { SortOrder } from "@/lib/query"

type OrderButtonProps = {
  readonly id: string
  readonly order: SortOrder
  readonly onChange: (order: SortOrder) => void
}

/**
 * Toggles a list between ascending and descending order. Lists order by
 * different fields, so the control shows the direction only and leaves the
 * field to the page that owns it.
 */
export function OrderButton({ id, order, onChange }: OrderButtonProps) {
  const ascending = order === "asc"

  return (
    <div className="flex flex-col gap-1.5">
      <Label htmlFor={id}>Order</Label>
      <Button
        id={id}
        variant="outline"
        size="icon"
        aria-label={ascending ? "Order ascending" : "Order descending"}
        title={ascending ? "Switch to descending order" : "Switch to ascending order"}
        onClick={() => onChange(ascending ? "desc" : "asc")}
      >
        {ascending ? <SortAscendingIcon /> : <SortDescendingIcon />}
      </Button>
    </div>
  )
}
