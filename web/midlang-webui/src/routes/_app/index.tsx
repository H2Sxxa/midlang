import { createFileRoute } from "@tanstack/react-router"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"

export const Route = createFileRoute("/_app/")({
  component: OverviewPage,
})

function OverviewPage() {
  return (
    <Card className="max-w-2xl">
      <CardHeader>
        <CardTitle>Overview</CardTitle>
        <CardDescription>
          The admin shell is wired up: a collapsible sidebar, a header showing the active
          section, and file-based routes under src/routes/_app.
        </CardDescription>
      </CardHeader>
      <CardContent className="text-muted-foreground space-y-2">
        <p>Add a page by creating a route file next to this one.</p>
        <p>
          Navigation entries live in lib/navigation.ts and the sidebar itself in
          components/app-sidebar.tsx.
        </p>
      </CardContent>
    </Card>
  )
}