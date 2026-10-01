import { ArrowLeftIcon, TrashIcon } from "@phosphor-icons/react"
import { createFileRoute, Link, useNavigate } from "@tanstack/react-router"
import { useState } from "react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { useTranslation } from "@/hooks/use-translation"
import { useAuth } from "@/hooks/use-auth"

export const Route = createFileRoute("/_app/translations/$locale/$key")({
  component: TranslationDetailPage,
})

function TranslationDetailPage() {
  const auth = useAuth()
  const { locale, key } = Route.useParams()
  const navigate = useNavigate()
  const translation = useTranslation({ locale, key, keyword: "", order: "asc" })
  const [error, setError] = useState<string | null>(null)

  const handleDelete = async () => {
    if (!window.confirm(`Delete “${key}”?`)) return

    setError(null)
    try {
      await translation.deleteMutation.mutateAsync({ locale, key })
      await navigate({ to: "/translations" })
    } catch (requestError) {
      setError(requestError instanceof Error ? requestError.message : "Unable to delete translation.")
    }
  }

  return (
    <div className="mx-auto w-full max-w-2xl space-y-6">
      <Button nativeButton={false} variant="ghost" render={<Link to="/translations" />}><ArrowLeftIcon /> Back to translations</Button>
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Translation detail</h1>
        <p className="mt-1 text-muted-foreground">Inspect or remove one translation resource.</p>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>{key}</CardTitle>
          <CardDescription>{locale}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {translation.detailQuery.isError ? (
            <p className="text-sm text-destructive">{translation.detailQuery.error.message}</p>
          ) : translation.detailQuery.isLoading ? (
            <p className="text-sm text-muted-foreground">Loading translation…</p>
          ) : (
            <div className="rounded-md border bg-muted/30 p-4 text-sm whitespace-pre-wrap">
              {translation.detailQuery.data?.value}
            </div>
          )}
          {error !== null && <p className="text-xs text-destructive">{error}</p>}
          {auth.can("translation:delete") && (
            <Button variant="destructive" disabled={translation.deleteMutation.isPending} onClick={() => void handleDelete()}>
              <TrashIcon /> {translation.deleteMutation.isPending ? "Deleting…" : "Delete translation"}
            </Button>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
