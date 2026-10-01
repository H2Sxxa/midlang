import { createFileRoute } from "@tanstack/react-router"
import { TodoPage } from "@/components/todo-page"

export const Route = createFileRoute("/_app/issues")({
  component: IssuesPage,
})

function IssuesPage() {
  return (
    <TodoPage
      title="Issues"
      description="Find missing translation keys reported by your applications."
      phase="M3 — Online diagnostics"
    />
  )
}
