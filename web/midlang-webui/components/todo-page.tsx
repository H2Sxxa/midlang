import { WrenchIcon } from "@phosphor-icons/react"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"

type TodoPageProps = {
  readonly title: string
  readonly description: string
  readonly phase: string
}

export function TodoPage({ title, description, phase }: TodoPageProps) {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
        <p className="mt-1 text-muted-foreground">{description}</p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <WrenchIcon className="size-4 text-muted-foreground" />
            Coming soon
          </CardTitle>
          <CardDescription>
            This page is part of the MidLang roadmap and has not been implemented yet.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="rounded-md border border-dashed p-4 text-sm text-muted-foreground">
            Planned for {phase}.
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
