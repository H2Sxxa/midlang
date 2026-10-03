import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import type { components } from "@/api/generated"
import { useClient } from "@/hooks/use-client"
import { useToast } from "@/hooks/use-toast"
import { describeApiError } from "@/lib/api-error"

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
  const toast = useToast()

  const groupsQuery = useQuery({
    queryKey: ["auth-groups", client],
    enabled: client !== null,
    queryFn: async () => {
      if (client === null) throw new Error("API client is unavailable")
      const result = await client.GET("/auth/groups")
      if (result.error !== undefined || result.data === undefined) {
        if (result.response.status === 401) {
          toast.error({ message: "Authentication expired", description: "Please sign in again." })
        }
        throw new Error(describeApiError(result.error, "Unable to load permission groups."))
      }
      return result.data.groups
    },
  })

  const catalogQuery = useQuery({
    queryKey: ["auth-permission-catalog", client],
    enabled: client !== null,
    queryFn: async () => {
      if (client === null) throw new Error("API client is unavailable")
      const result = await client.GET("/auth/permissions/catalog")
      if (result.error !== undefined || result.data === undefined) {
        throw new Error(describeApiError(result.error, "Unable to load the permission catalog."))
      }
      return result.data.permissions
    },
  })

  const invalidate = async () => {
    await queryClient.invalidateQueries({ queryKey: ["auth-groups"] })
    await queryClient.invalidateQueries({ queryKey: ["auth-tokens"] })
  }

  const createMutation = useMutation({
    mutationFn: async (values: GroupFormValues) => {
      if (client === null) throw new Error("API client is unavailable")
      const result = await client.POST("/auth/groups", {
        body: {
          name: values.name,
          description: values.description,
          permissions: [...values.permissions],
        },
      })
      if (result.error !== undefined || result.data === undefined) {
        throw new Error(describeApiError(result.error, "Unable to create the group."))
      }
      return result.data
    },
    onSuccess: invalidate,
  })

  const updateMutation = useMutation({
    mutationFn: async ({ name, values }: UpdateGroupInput) => {
      if (client === null) throw new Error("API client is unavailable")
      const result = await client.PUT("/auth/groups/{name}", {
        params: { path: { name } },
        body: {
          description: values.description,
          permissions: [...values.permissions],
        },
      })
      if (result.error !== undefined || result.data === undefined) {
        throw new Error(describeApiError(result.error, "Unable to update the group."))
      }
      return result.data
    },
    onSuccess: invalidate,
  })

  const deleteMutation = useMutation({
    mutationFn: async (name: string) => {
      if (client === null) throw new Error("API client is unavailable")
      const result = await client.DELETE("/auth/groups/{name}", {
        params: { path: { name } },
      })
      if (result.response.status !== 204) {
        throw new Error(describeApiError(result.error, "Unable to delete the group."))
      }
    },
    onSuccess: invalidate,
  })

  return { groupsQuery, catalogQuery, createMutation, updateMutation, deleteMutation }
}
