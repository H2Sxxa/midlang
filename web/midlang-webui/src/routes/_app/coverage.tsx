import { createFileRoute } from "@tanstack/react-router"
import { TodoPage } from "@/components/todo-page"

export const Route = createFileRoute("/_app/coverage")({
  component: CoveragePage,
})

function CoveragePage() {
  return (
    <TodoPage
      title="Coverage"
      description="Track translation coverage by locale against a reference locale."
      phase="M3 — Online diagnostics"
    />
  )
}
