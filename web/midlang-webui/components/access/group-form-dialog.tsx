import { useState, type SubmitEvent } from "react"
import { PermissionPicker } from "@/components/access/permission-picker"
import { Button } from "@/components/ui/button"
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
import { Textarea } from "@/components/ui/textarea"
import type { GroupFormValues, PermissionInfo } from "@/hooks/use-permission-groups"

type GroupFormDialogProps = {
  readonly open: boolean
  readonly onOpenChange: (open: boolean) => void
  readonly title: string
  readonly description: string
  readonly submitLabel: string
  readonly formKey: string
  readonly permissions: ReadonlyArray<PermissionInfo>
  readonly initialValues: GroupFormValues
  readonly nameEditable: boolean
  readonly pending: boolean
  readonly error: string | null
  readonly onSubmit: (values: GroupFormValues) => void
}

export function GroupFormDialog(props: GroupFormDialogProps) {
  return (
    <Dialog open={props.open} onOpenChange={props.onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        {props.open && <GroupForm key={props.formKey} {...props} />}
      </DialogContent>
    </Dialog>
  )
}

function GroupForm({
  onOpenChange,
  title,
  description,
  submitLabel,
  permissions,
  initialValues,
  nameEditable,
  pending,
  error,
  onSubmit,
}: GroupFormDialogProps) {
  const [name, setName] = useState(initialValues.name)
  const [groupDescription, setGroupDescription] = useState(initialValues.description)
  const [selectedPermissions, setSelectedPermissions] = useState<ReadonlyArray<string>>(
    initialValues.permissions,
  )
  const [validationError, setValidationError] = useState<string | null>(null)

  const togglePermission = (permission: string) => {
    setSelectedPermissions((current) =>
      current.includes(permission)
        ? current.filter((item) => item !== permission)
        : [...current, permission],
    )
  }

  const handleSubmit = (event: SubmitEvent) => {
    event.preventDefault()
    const trimmedName = name.trim()
    if (nameEditable && trimmedName === "") {
      setValidationError("Group name is required.")
      return
    }
    setValidationError(null)
    onSubmit({
      name: trimmedName,
      description: groupDescription.trim(),
      permissions: selectedPermissions,
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
          <Label htmlFor="group-name">Name</Label>
          <Input
            id="group-name"
            placeholder="content-reviewer"
            value={name}
            disabled={!nameEditable}
            onChange={(event) => setName(event.target.value)}
          />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="group-description">Description</Label>
          <Textarea
            id="group-description"
            placeholder="What this group is allowed to do."
            value={groupDescription}
            onChange={(event) => setGroupDescription(event.target.value)}
          />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label>Permissions</Label>
          <PermissionPicker
            permissions={permissions}
            selected={selectedPermissions}
            onToggle={togglePermission}
            disabled={pending}
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
