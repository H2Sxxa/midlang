import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import type { components } from "@/api/generated"
import { useClient } from "@/hooks/use-client"

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

  const tokensQuery = useQuery({
    queryKey: ["auth-tokens", client],
    queryFn: async () => (await client.GET("/auth/tokens")).data!.tokens,
  })

  const groupsQuery = useQuery({
    queryKey: ["auth-groups", client],
    queryFn: async () => (await client.GET("/auth/groups")).data!.groups,
  })

  const invalidate = async () => {
    await queryClient.invalidateQueries({ queryKey: ["auth-tokens"] })
  }

  const createMutation = useMutation({
    mutationFn: async (values: TokenFormValues) => {
      const result = await client.POST("/auth/tokens", {
        body: {
          name: values.name,
          groups: [...values.groups],
          expires_at: values.expiresAt,
        },
      })
      return result.data!
    },
    onSuccess: invalidate,
  })

  const updateMutation = useMutation({
    mutationFn: async ({ id, values }: UpdateTokenInput) => {
      const result = await client.PATCH("/auth/tokens/{id}", {
        params: { path: { id } },
        body: {
          name: values.name,
          groups: [...values.groups],
          expires_at: values.expiresAt,
        },
      })
      return result.data!
    },
    onSuccess: invalidate,
  })

  const revokeMutation = useMutation({
    mutationFn: (id: string) =>
      client.DELETE("/auth/tokens/{id}", { params: { path: { id } } }),
    onSuccess: invalidate,
  })

  const rotateMutation = useMutation({
    mutationFn: async (id: string) => {
      const result = await client.POST("/auth/tokens/{id}/rotate", { params: { path: { id } } })
      return result.data!
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
