import type { PermissionInfo } from "@/hooks/use-permission-groups"

/**
 * Expands a set of granted permissions with everything they imply. The catalog
 * reports implications transitively, so a single pass is enough.
 */
export function effectivePermissions(
  granted: ReadonlyArray<string>,
  catalog: ReadonlyArray<PermissionInfo>,
): ReadonlySet<string> {
  const impliesByName = new Map(
    catalog.map((permission) => [permission.name, permission.implies]),
  )
  const effective = new Set<string>()
  for (const name of granted) {
    effective.add(name)
    for (const implied of impliesByName.get(name) ?? []) {
      effective.add(implied)
    }
  }
  return effective
}
