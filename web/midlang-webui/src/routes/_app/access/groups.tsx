import { PencilSimpleIcon, PlusIcon, TrashIcon } from "@phosphor-icons/react"
import { createFileRoute } from "@tanstack/react-router"
import { useState } from "react"
import { ConfirmDialog } from "@/components/access/confirm-dialog"
import { GroupFormDialog } from "@/components/access/group-form-dialog"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { useAuth } from "@/hooks/use-auth"
import { useToast } from "@/hooks/use-toast"
import {
  usePermissionGroups,
  type GroupFormValues,
  type GroupInfo,
} from "@/hooks/use-permission-groups"
import { formatTimestamp } from "@/lib/format"

export const Route = createFileRoute("/_app/access/groups")({
  component: GroupsPage,
})

type FormState =
  | { readonly mode: "create" }
  | { readonly mode: "edit"; readonly group: GroupInfo }

function PermissionBadges({ permissions }: { readonly permissions: ReadonlyArray<string> }) {
  const visible = permissions.slice(0, 4)
  const hidden = permissions.length - visible.length

  return (
    <div className="flex max-w-md flex-wrap items-center gap-1">
      {visible.map((permission) => (
        <Badge key={permission} variant="outline">
          {permission}
        </Badge>
      ))}
      {hidden > 0 && (
        <Badge variant="secondary" title={permissions.join(", ")}>
          +{hidden}
        </Badge>
      )}
      {permissions.length === 0 && <span className="text-muted-foreground">No permissions</span>}
    </div>
  )
}

function GroupsPage() {
  const auth = useAuth()
  const toast = useToast()
  const access = usePermissionGroups()
  const [formState, setFormState] = useState<FormState | null>(null)
  const [formError, setFormError] = useState<string | null>(null)
  const [pendingDelete, setPendingDelete] = useState<GroupInfo | null>(null)

  const canRead = auth.can("permission:read")
  const canManage = auth.can("permission:manage")
  const groups = access.groupsQuery.data ?? []
  const catalog = access.catalogQuery.data ?? []
  const saving = access.createMutation.isPending || access.updateMutation.isPending

  const openCreate = () => {
    setFormError(null)
    setFormState({ mode: "create" })
  }

  const openEdit = (group: GroupInfo) => {
    setFormError(null)
    setFormState({ mode: "edit", group })
  }

  const handleSubmit = async (values: GroupFormValues) => {
    setFormError(null)
    try {
      if (formState?.mode === "edit") {
        await access.updateMutation.mutateAsync({ name: formState.group.name, values })
        toast.success({ message: "Group updated", description: formState.group.name })
      } else {
        await access.createMutation.mutateAsync(values)
        toast.success({ message: "Group created", description: values.name })
      }
      setFormState(null)
    } catch (error) {
      setFormError(error instanceof Error ? error.message : "Unable to save the group.")
    }
  }

  const handleDelete = async () => {
    if (pendingDelete === null) {
      return
    }
    try {
      await access.deleteMutation.mutateAsync(pendingDelete.name)
      toast.success({ message: "Group deleted", description: pendingDelete.name })
      setPendingDelete(null)
    } catch (error) {
      toast.error({
        message: "Delete failed",
        description: error instanceof Error ? error.message : "Unable to delete the group.",
      })
    }
  }

  if (!canRead) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Permission groups unavailable</CardTitle>
          <CardDescription>
            You need the permission:read permission to view permission groups.
          </CardDescription>
        </CardHeader>
      </Card>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex items-end justify-between gap-4">
        <div>
          <h2 className="text-lg font-semibold tracking-tight">Permission groups</h2>
          <p className="mt-1 text-muted-foreground">
            Bundle permissions once, then attach the bundle to any number of tokens.
          </p>
        </div>
        {canManage && (
          <Button onClick={openCreate}>
            <PlusIcon /> Create group
          </Button>
        )}
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Groups</CardTitle>
          <CardDescription>{groups.length} group(s)</CardDescription>
        </CardHeader>
        <CardContent>
          {access.groupsQuery.isError ? (
            <p className="py-8 text-sm text-destructive">{access.groupsQuery.error.message}</p>
          ) : access.groupsQuery.isLoading ? (
            <p className="py-8 text-sm text-muted-foreground">Loading permission groups…</p>
          ) : (
            <div className="rounded-md border">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Name</TableHead>
                    <TableHead>Description</TableHead>
                    <TableHead>Type</TableHead>
                    <TableHead>Permissions</TableHead>
                    <TableHead>Created</TableHead>
                    <TableHead className="w-20 text-right">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {groups.map((group) => (
                    <TableRow key={group.name}>
                      <TableCell className="font-medium">{group.name}</TableCell>
                      <TableCell className="max-w-xs whitespace-normal text-muted-foreground">
                        {group.description}
                      </TableCell>
                      <TableCell>
                        <Badge variant={group.built_in ? "outline" : "secondary"}>
                          {group.built_in ? "Built-in" : "Custom"}
                        </Badge>
                      </TableCell>
                      <TableCell>
                        <PermissionBadges permissions={group.permissions} />
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {formatTimestamp(group.created_at)}
                      </TableCell>
                      <TableCell className="text-right">
                        {canManage && !group.built_in && (
                          <div className="flex justify-end gap-1">
                            <Button
                              size="icon-sm"
                              variant="ghost"
                              aria-label={`Edit ${group.name}`}
                              onClick={() => openEdit(group)}
                            >
                              <PencilSimpleIcon />
                            </Button>
                            <Button
                              size="icon-sm"
                              variant="ghost"
                              className="text-destructive hover:text-destructive"
                              aria-label={`Delete ${group.name}`}
                              onClick={() => setPendingDelete(group)}
                            >
                              <TrashIcon />
                            </Button>
                          </div>
                        )}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>

      <GroupFormDialog
        open={formState !== null}
        onOpenChange={(open) => {
          if (!open) setFormState(null)
        }}
        title={formState?.mode === "edit" ? "Edit permission group" : "Create permission group"}
        description={
          formState?.mode === "edit"
            ? "Update which permissions this group grants."
            : "Members on this group receive every permission you select."
        }
        submitLabel={formState?.mode === "edit" ? "Save changes" : "Create group"}
        formKey={formState?.mode === "edit" ? formState.group.name : "create"}
        permissions={catalog}
        initialValues={
          formState?.mode === "edit"
            ? {
                name: formState.group.name,
                description: formState.group.description,
                permissions: formState.group.permissions,
              }
            : { name: "", description: "", permissions: [] }
        }
        nameEditable={formState?.mode !== "edit"}
        pending={saving}
        error={formError}
        onSubmit={(values) => void handleSubmit(values)}
      />

      <ConfirmDialog
        open={pendingDelete !== null}
        onOpenChange={(open) => {
          if (!open) setPendingDelete(null)
        }}
        title="Delete permission group"
        description={`Delete “${pendingDelete?.name ?? ""}”? Tokens still using it must be reassigned first.`}
        confirmLabel="Delete"
        destructive
        pending={access.deleteMutation.isPending}
        onConfirm={() => void handleDelete()}
      />
    </div>
  )
}
