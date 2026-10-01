import { createFileRoute } from "@tanstack/react-router"
import { TodoPage } from "@/components/todo-page"

export const Route = createFileRoute("/_app/exports")({
  component: ExportsPage,
})

function ExportsPage() {
  return (
    <TodoPage
      title="Exports"
      description="Generate deterministic artifacts from a versioned translation snapshot."
      phase="M4 — Versioned export"
    />
  )
}
