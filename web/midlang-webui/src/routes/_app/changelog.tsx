import { createFileRoute } from "@tanstack/react-router"

export const Route = createFileRoute("/_app/changelog")({
  component: RouteComponent,
})

function RouteComponent() {
  return <div>Hello "/_app/changelog"!</div>
}
