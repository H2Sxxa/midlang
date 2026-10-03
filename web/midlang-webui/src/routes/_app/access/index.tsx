import { createFileRoute, redirect } from "@tanstack/react-router"

export const Route = createFileRoute("/_app/access/")({
  beforeLoad: ({ context }) => {
    const canReadTokens = context.auth?.can("token:read") ?? false
    throw redirect({ to: canReadTokens ? "/access/tokens" : "/access/groups", replace: true })
  },
})
