import { useState, type SubmitEvent } from "react"
import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import type { AccessGroup, TokenFormValues } from "@/hooks/use-tokens"
import { epochSecondsToLocalInput, localInputToEpochSeconds } from "@/lib/format"

type TokenFormDialogProps = {
  readonly open: boolean
  readonly onOpenChange: (open: boolean) => void
  readonly title: string
  readonly description: string
  readonly submitLabel: string
  readonly formKey: string
  readonly groups: ReadonlyArray<AccessGroup>
  readonly initialValues: TokenFormValues
  readonly pending: boolean
  readonly error: string | null
  readonly onSubmit: (values: TokenFormValues) => void
}

export function TokenFormDialog(props: TokenFormDialogProps) {
  return (
    <Dialog open={props.open} onOpenChange={props.onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        {props.open && <TokenForm key={props.formKey} {...props} />}
      </DialogContent>
    </Dialog>
  )
}

function TokenForm({
  onOpenChange,
  title,
  description,
  submitLabel,
  groups,
  initialValues,
  pending,
  error,
  onSubmit,
}: TokenFormDialogProps) {
  const [name, setName] = useState(initialValues.name)
  const [selectedGroups, setSelectedGroups] = useState<ReadonlyArray<string>>(initialValues.groups)
  const [expiry, setExpiry] = useState(epochSecondsToLocalInput(initialValues.expiresAt))
  const [validationError, setValidationError] = useState<string | null>(null)

  const toggleGroup = (group: string) => {
    setSelectedGroups((current) =>
      current.includes(group)
        ? current.filter((item) => item !== group)
        : [...current, group],
    )
  }

  const handleSubmit = (event: SubmitEvent) => {
    event.preventDefault()
    const trimmedName = name.trim()
    if (trimmedName === "") {
      setValidationError("Token name is required.")
      return
    }
    if (selectedGroups.length === 0) {
      setValidationError("Select at least one permission group.")
      return
    }
    setValidationError(null)
    onSubmit({
      name: trimmedName,
      groups: selectedGroups,
      expiresAt: localInputToEpochSeconds(expiry),
    })
  }

  const message = validationError ?? error

  return (
    <>
      <DialogHeader>
        <DialogTitle>{title}</DialogTitle>
        <DialogDescription>{description}</DialogDescription>
      </DialogHeader>
      <form className="space-y-4" noValidate onSubmit={handleSubmit}>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="token-name">Name</Label>
          <Input
            id="token-name"
            placeholder="ci-deploy"
            value={name}
            onChange={(event) => setName(event.target.value)}
          />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label>Permission groups</Label>
          <div className="max-h-40 space-y-1.5 overflow-y-auto border p-3">
            {groups.map((group) => (
              <label key={group.name} className="flex items-start gap-2">
                <Checkbox
                  className="mt-0.5"
                  checked={selectedGroups.includes(group.name)}
                  onCheckedChange={() => toggleGroup(group.name)}
                />
                <span className="flex flex-col">
                  <span className="font-medium">{group.name}</span>
                  <span className="text-muted-foreground">{group.description}</span>
                </span>
              </label>
            ))}
            {groups.length === 0 && (
              <p className="text-muted-foreground">No permission groups available.</p>
            )}
          </div>
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="token-expiry">Expires at (optional)</Label>
          <Input
            id="token-expiry"
            type="datetime-local"
            value={expiry}
            onChange={(event) => setExpiry(event.target.value)}
          />
        </div>
        {message !== null && <p className="text-xs text-destructive">{message}</p>}
        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={pending}
          >
            Cancel
          </Button>
          <Button type="submit" disabled={pending}>
            {pending ? "Saving…" : submitLabel}
          </Button>
        </DialogFooter>
      </form>
    </>
  )
}
