import { createFileRoute } from "@tanstack/react-router"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"

export const Route = createFileRoute("/_app/settings")({
  component: SettingsPage,
})

function SettingsPage() {
  return (
    <Card className="max-w-2xl">
      <CardHeader>
        <CardTitle>Settings</CardTitle>
        <CardDescription>Console and workspace preferences.</CardDescription>
      </CardHeader>
      <CardContent className="text-muted-foreground">
        No settings are wired up yet.
      </CardContent>
    </Card>
  )
}