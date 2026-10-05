import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import type { components } from "@/api/generated"
import { useClient } from "@/hooks/use-client"

export type GroupInfo = components["schemas"]["GroupInfo"]
export type PermissionInfo = components["schemas"]["PermissionInfo"]

export type GroupFormValues = {
  readonly name: string
  readonly description: string
  readonly permissions: ReadonlyArray<string>
}

type UpdateGroupInput = {
  readonly name: string
  readonly values: GroupFormValues
}

export function usePermissionGroups() {
  const client = useClient()
  const queryClient = useQueryClient()

  const groupsQuery = useQuery({
    queryKey: ["auth-groups", client],
    queryFn: async () => (await client.GET("/auth/groups")).data!.groups,
  })

  const catalogQuery = useQuery({
    queryKey: ["auth-permission-catalog", client],
    queryFn: async () => (await client.GET("/auth/permissions/catalog")).data!.permissions,
  })

  const invalidate = async () => {
    await queryClient.invalidateQueries({ queryKey: ["auth-groups"] })
    await queryClient.invalidateQueries({ queryKey: ["auth-tokens"] })
  }

  const createMutation = useMutation({
    mutationFn: async (values: GroupFormValues) => {
      const result = await client.POST("/auth/groups", {
        body: {
          name: values.name,
          description: values.description,
          permissions: [...values.permissions],
        },
      })
      return result.data!
    },
    onSuccess: invalidate,
  })

  const updateMutation = useMutation({
    mutationFn: async ({ name, values }: UpdateGroupInput) => {
      const result = await client.PUT("/auth/groups/{name}", {
        params: { path: { name } },
        body: {
          description: values.description,
          permissions: [...values.permissions],
        },
      })
      return result.data!
    },
    onSuccess: invalidate,
  })

  const deleteMutation = useMutation({
    mutationFn: (name: string) =>
      client.DELETE("/auth/groups/{name}", { params: { path: { name } } }),
    onSuccess: invalidate,
  })

  return { groupsQuery, catalogQuery, createMutation, updateMutation, deleteMutation }
}
