import {
  ArrowsClockwiseIcon,
  PencilSimpleIcon,
  PlusIcon,
  ProhibitIcon,
} from "@phosphor-icons/react"
import { createFileRoute } from "@tanstack/react-router"
import { useState } from "react"
import { ConfirmDialog } from "@/components/access/confirm-dialog"
import { TokenFormDialog } from "@/components/access/token-form-dialog"
import { TokenSecretDialog } from "@/components/access/token-secret-dialog"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { useAuth } from "@/hooks/use-auth"
import { useToast } from "@/hooks/use-toast"
import {
  useTokens,
  type CreatedToken,
  type TokenFormValues,
  type TokenInfo,
} from "@/hooks/use-tokens"
import { formatTimestamp } from "@/lib/format"

export const Route = createFileRoute("/_app/access/tokens")({
  component: TokensPage,
})

type FormState =
  | { readonly mode: "create" }
  | { readonly mode: "edit"; readonly token: TokenInfo }

function tokenStatusVariant(status: string): "default" | "destructive" | "outline" {
  if (status === "active") {
    return "default"
  }
  if (status === "revoked") {
    return "destructive"
  }
  return "outline"
}

function TokensPage() {
  const auth = useAuth()
  const toast = useToast()
  const access = useTokens()
  const [formState, setFormState] = useState<FormState | null>(null)
  const [formError, setFormError] = useState<string | null>(null)
  const [secret, setSecret] = useState<CreatedToken | null>(null)
  const [pendingRevoke, setPendingRevoke] = useState<TokenInfo | null>(null)

  const canRead = auth.can("token:read")
  const tokens = access.tokensQuery.data ?? []
  const groups = access.groupsQuery.data ?? []
  const saving = access.createMutation.isPending || access.updateMutation.isPending

  const openCreate = () => {
    setFormError(null)
    setFormState({ mode: "create" })
  }

  const openEdit = (token: TokenInfo) => {
    setFormError(null)
    setFormState({ mode: "edit", token })
  }

  const handleSubmit = async (values: TokenFormValues) => {
    setFormError(null)
    try {
      if (formState?.mode === "edit") {
        await access.updateMutation.mutateAsync({ id: formState.token.id, values })
        toast.success({ message: "Token updated", description: values.name })
      } else {
        const created = await access.createMutation.mutateAsync(values)
        setSecret(created)
      }
      setFormState(null)
    } catch (error) {
      setFormError(error instanceof Error ? error.message : "Unable to save the token.")
    }
  }

  const handleRotate = (token: TokenInfo) => {
    access.rotateMutation.mutate(token.id, {
      onSuccess: (rotated) => setSecret(rotated),
    })
  }

  const handleRevoke = () => {
    if (pendingRevoke === null) {
      return
    }
    const token = pendingRevoke
    access.revokeMutation.mutate(token.id, {
      onSuccess: () => {
        toast.success({ message: "Token revoked", description: token.name })
        setPendingRevoke(null)
      },
    })
  }

  if (!canRead) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Tokens unavailable</CardTitle>
          <CardDescription>You need the token:read permission to view API tokens.</CardDescription>
        </CardHeader>
      </Card>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex items-end justify-between gap-4">
        <div>
          <h2 className="text-lg font-semibold tracking-tight">API tokens</h2>
          <p className="mt-1 text-muted-foreground">
            Issue and revoke the bearer tokens used to call this server.
          </p>
        </div>
        {auth.can("token:create") && (
          <Button onClick={openCreate}>
            <PlusIcon /> Create token
          </Button>
        )}
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Issued tokens</CardTitle>
          <CardDescription>{tokens.length} token(s)</CardDescription>
        </CardHeader>
        <CardContent>
          {access.tokensQuery.isError ? (
            <p className="py-8 text-sm text-destructive">{access.tokensQuery.error.message}</p>
          ) : access.tokensQuery.isLoading ? (
            <p className="py-8 text-sm text-muted-foreground">Loading tokens…</p>
          ) : tokens.length === 0 ? (
            <p className="py-8 text-sm text-muted-foreground">No tokens have been issued yet.</p>
          ) : (
            <div className="rounded-md border">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Name</TableHead>
                    <TableHead>Prefix</TableHead>
                    <TableHead>Groups</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Created</TableHead>
                    <TableHead>Last used</TableHead>
                    <TableHead>Expires</TableHead>
                    <TableHead className="w-28 text-right">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {tokens.map((token) => (
                    <TableRow key={token.id}>
                      <TableCell className="font-medium">{token.name}</TableCell>
                      <TableCell className="font-mono text-muted-foreground">
                        {token.token_prefix}
                      </TableCell>
                      <TableCell>
                        <div className="flex flex-wrap gap-1">
                          {token.groups.map((group) => (
                            <Badge key={group} variant="secondary">
                              {group}
                            </Badge>
                          ))}
                        </div>
                      </TableCell>
                      <TableCell>
                        <Badge variant={tokenStatusVariant(token.status)}>{token.status}</Badge>
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {formatTimestamp(token.created_at)}
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {formatTimestamp(token.last_used_at)}
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {formatTimestamp(token.expires_at)}
                      </TableCell>
                      <TableCell className="text-right">
                        <div className="flex justify-end gap-1">
                          {auth.can("token:update") && token.status === "active" && (
                            <Button
                              size="icon-sm"
                              variant="ghost"
                              aria-label={`Edit ${token.name}`}
                              onClick={() => openEdit(token)}
                            >
                              <PencilSimpleIcon />
                            </Button>
                          )}
                          {auth.can("token:rotate") && token.status === "active" && (
                            <Button
                              size="icon-sm"
                              variant="ghost"
                              aria-label={`Rotate ${token.name}`}
                              disabled={access.rotateMutation.isPending}
                              onClick={() => void handleRotate(token)}
                            >
                              <ArrowsClockwiseIcon />
                            </Button>
                          )}
                          {auth.can("token:revoke") && token.status === "active" && (
                            <Button
                              size="icon-sm"
                              variant="ghost"
                              className="text-destructive hover:text-destructive"
                              aria-label={`Revoke ${token.name}`}
                              onClick={() => setPendingRevoke(token)}
                            >
                              <ProhibitIcon />
                            </Button>
                          )}
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>

      <TokenFormDialog
        open={formState !== null}
        onOpenChange={(open) => {
          if (!open) setFormState(null)
        }}
        title={formState?.mode === "edit" ? "Edit token" : "Create token"}
        description={
          formState?.mode === "edit"
            ? "Change the token name, groups or expiry. The secret stays the same."
            : "Pick the permission groups this token should inherit."
        }
        submitLabel={formState?.mode === "edit" ? "Save changes" : "Create token"}
        formKey={formState?.mode === "edit" ? formState.token.id : "create"}
        groups={groups}
        initialValues={
          formState?.mode === "edit"
            ? {
                name: formState.token.name,
                groups: formState.token.groups,
                expiresAt: formState.token.expires_at ?? null,
              }
            : { name: "", groups: [], expiresAt: null }
        }
        pending={saving}
        error={formError}
        onSubmit={(values) => void handleSubmit(values)}
      />

      {secret !== null && (
        <TokenSecretDialog token={secret} onOpenChange={(open) => !open && setSecret(null)} />
      )}

      <ConfirmDialog
        open={pendingRevoke !== null}
        onOpenChange={(open) => {
          if (!open) setPendingRevoke(null)
        }}
        title="Revoke token"
        description={`Revoke “${pendingRevoke?.name ?? ""}”? Clients using it will immediately lose access.`}
        confirmLabel="Revoke"
        destructive
        pending={access.revokeMutation.isPending}
        onConfirm={() => void handleRevoke()}
      />
    </div>
  )
}
