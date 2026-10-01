import { createFileRoute } from "@tanstack/react-router"
import { TodoPage } from "@/components/todo-page"

export const Route = createFileRoute("/_app/versions")({
  component: VersionsPage,
})

function VersionsPage() {
  return (
    <TodoPage
      title="Versions"
      description="Pin immutable translation snapshots for each release."
      phase="M4 — Versioned export"
    />
  )
}
