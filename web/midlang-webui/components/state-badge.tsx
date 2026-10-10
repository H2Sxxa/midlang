import { Badge } from "@/components/ui/badge"

type StateVariant = "default" | "secondary" | "destructive" | "outline"

/**
 * Tone per lifecycle state, so the same kind of state looks the same whatever
 * page shows it: active or newly created is `default`, settled is `secondary`,
 * removed is `destructive` and set aside is `outline`.
 */
const STATE_VARIANTS: Readonly<Record<string, StateVariant>> = {
  open: "default",
  created: "default",
  closed: "secondary",
  updated: "secondary",
  deleted: "destructive",
  ignored: "outline",
}

/** Renders a lifecycle state as a badge with the tone that state means. */
export function StateBadge({ state }: { readonly state: string }) {
  return <Badge variant={STATE_VARIANTS[state] ?? "outline"}>{state}</Badge>
}
