import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import type { components } from "@/api/generated"
import { useClient } from "@/hooks/use-client"
import { useToast } from "@/hooks/use-toast"
import { describeApiError } from "@/lib/api-error"

export type TokenInfo = components["schemas"]["TokenInfo"]
export type CreatedToken = components["schemas"]["CreatedToken"]
export type AccessGroup = components["schemas"]["GroupInfo"]

export type TokenFormValues = {
  readonly name: string
  readonly groups: ReadonlyArray<string>
  readonly expiresAt: number | null
}

type UpdateTokenInput = {
  readonly id: string
  readonly values: TokenFormValues
}

export function useTokens() {
  const client = useClient()
  const queryClient = useQueryClient()
  const toast = useToast()

  const tokensQuery = useQuery({
    queryKey: ["auth-tokens", client],
    enabled: client !== null,
    queryFn: async () => {
      if (client === null) throw new Error("API client is unavailable")
      const result = await client.GET("/auth/tokens")
      if (result.error !== undefined || result.data === undefined) {
        if (result.response.status === 401) {
          toast.error({ message: "Authentication expired", description: "Please sign in again." })
        }
        throw new Error(describeApiError(result.error, "Unable to load tokens."))
      }
      return result.data.tokens
    },
  })

  const groupsQuery = useQuery({
    queryKey: ["auth-groups", client],
    enabled: client !== null,
    queryFn: async () => {
      if (client === null) throw new Error("API client is unavailable")
      const result = await client.GET("/auth/groups")
      if (result.error !== undefined || result.data === undefined) {
        throw new Error(describeApiError(result.error, "Unable to load permission groups."))
      }
      return result.data.groups
    },
  })

  const invalidate = async () => {
    await queryClient.invalidateQueries({ queryKey: ["auth-tokens"] })
  }

  const createMutation = useMutation({
    mutationFn: async (values: TokenFormValues) => {
      if (client === null) throw new Error("API client is unavailable")
      const result = await client.POST("/auth/tokens", {
        body: {
          name: values.name,
          groups: [...values.groups],
          expires_at: values.expiresAt,
        },
      })
      if (result.error !== undefined || result.data === undefined) {
        throw new Error(describeApiError(result.error, "Unable to create the token."))
      }
      return result.data
    },
    onSuccess: invalidate,
  })

  const updateMutation = useMutation({
    mutationFn: async ({ id, values }: UpdateTokenInput) => {
      if (client === null) throw new Error("API client is unavailable")
      const result = await client.PATCH("/auth/tokens/{id}", {
        params: { path: { id } },
        body: {
          name: values.name,
          groups: [...values.groups],
          expires_at: values.expiresAt,
        },
      })
      if (result.error !== undefined || result.data === undefined) {
        throw new Error(describeApiError(result.error, "Unable to update the token."))
      }
      return result.data
    },
    onSuccess: invalidate,
  })

  const revokeMutation = useMutation({
    mutationFn: async (id: string) => {
      if (client === null) throw new Error("API client is unavailable")
      const result = await client.DELETE("/auth/tokens/{id}", { params: { path: { id } } })
      if (result.response.status !== 204) {
        throw new Error(describeApiError(result.error, "Unable to revoke the token."))
      }
    },
    onSuccess: invalidate,
  })

  const rotateMutation = useMutation({
    mutationFn: async (id: string) => {
      if (client === null) throw new Error("API client is unavailable")
      const result = await client.POST("/auth/tokens/{id}/rotate", {
        params: { path: { id } },
      })
      if (result.error !== undefined || result.data === undefined) {
        throw new Error(describeApiError(result.error, "Unable to rotate the token."))
      }
      return result.data
    },
    onSuccess: invalidate,
  })

  return {
    tokensQuery,
    groupsQuery,
    createMutation,
    updateMutation,
    revokeMutation,
    rotateMutation,
  }
}
