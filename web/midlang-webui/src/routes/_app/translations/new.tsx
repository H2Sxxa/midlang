import { ArrowLeftIcon, FloppyDiskIcon } from "@phosphor-icons/react"
import { createFileRoute, Link, useNavigate } from "@tanstack/react-router"
import { useState, type SubmitEvent } from "react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { useTranslation } from "@/hooks/use-translation"

export const Route = createFileRoute("/_app/translations/new")({
  component: NewTranslationPage,
})

function NewTranslationPage() {
  const navigate = useNavigate()
  const [locale, setLocale] = useState("")
  const [key, setKey] = useState("")
  const [value, setValue] = useState("")
  const [error, setError] = useState<string | null>(null)
  const translation = useTranslation({ locale, keyword: "", order: "asc" })

  const handleSubmit = async (event: SubmitEvent) => {
    event.preventDefault()
    const input = { locale: locale.trim(), key: key.trim(), value: value.trim() }
    if (input.locale === "" || input.key === "") {
      setError("Locale and key are required.")
      return
    }

    setError(null)
    try {
      await translation.saveMutation.mutateAsync(input)
      await navigate({
        to: "/translations/$locale/$key",
        params: { locale: input.locale, key: input.key },
      })
    } catch (requestError) {
      setError(requestError instanceof Error ? requestError.message : "Unable to save translation.")
    }
  }

  return (
    <div className="mx-auto w-full max-w-2xl space-y-6">
      <Button variant="ghost" render={<Link to="/translations" />}><ArrowLeftIcon /> Back to translations</Button>
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Add translation</h1>
        <p className="mt-1 text-muted-foreground">Create a translation resource for a locale.</p>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>Translation resource</CardTitle>
          <CardDescription>The same form also updates an existing key.</CardDescription>
        </CardHeader>
        <CardContent>
          <form className="space-y-4" onSubmit={handleSubmit}>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="locale">Locale</Label>
              <Input id="locale" placeholder="en-US" value={locale} onChange={(event) => setLocale(event.target.value)} />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="key">Key</Label>
              <Input id="key" placeholder="checkout.title" value={key} onChange={(event) => setKey(event.target.value)} />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="value">Value</Label>
              <Input id="value" placeholder="Checkout" value={value} onChange={(event) => setValue(event.target.value)} />
            </div>
            {error !== null && <p className="text-xs text-destructive">{error}</p>}
            <Button type="submit" disabled={translation.saveMutation.isPending}>
              <FloppyDiskIcon /> {translation.saveMutation.isPending ? "Saving…" : "Save translation"}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  )
}
