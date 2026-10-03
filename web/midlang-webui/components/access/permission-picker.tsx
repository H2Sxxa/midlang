import { Checkbox } from "@/components/ui/checkbox"
import type { PermissionInfo } from "@/hooks/use-permission-groups"
import { effectivePermissions } from "@/lib/permissions"

type PermissionPickerProps = {
  readonly permissions: ReadonlyArray<PermissionInfo>
  readonly selected: ReadonlyArray<string>
  readonly onToggle: (permission: string) => void
  readonly disabled: boolean
}

type PermissionSection = {
  readonly prefix: string
  readonly items: ReadonlyArray<PermissionInfo>
}

function groupByPrefix(permissions: ReadonlyArray<PermissionInfo>): ReadonlyArray<PermissionSection> {
  const sections = new Map<string, PermissionInfo[]>()
  for (const permission of permissions) {
    const separator = permission.name.indexOf(":")
    const prefix = separator === -1 ? "other" : permission.name.slice(0, separator)
    const bucket = sections.get(prefix) ?? []
    bucket.push(permission)
    sections.set(prefix, bucket)
  }
  return [...sections.entries()].map(([prefix, items]) => ({ prefix, items }))
}

export function PermissionPicker({ permissions, selected, onToggle, disabled }: PermissionPickerProps) {
  const sections = groupByPrefix(permissions)
  const effective = effectivePermissions(selected, permissions)

  return (
    <div className="max-h-64 space-y-4 overflow-y-auto border p-3">
      {sections.map((section) => (
        <div key={section.prefix} className="space-y-2">
          <p className="text-xs font-medium tracking-wide text-muted-foreground uppercase">
            {section.prefix}
          </p>
          <div className="space-y-1.5">
            {section.items.map((permission) => {
              const isGranted = selected.includes(permission.name)
              const isImplied = !isGranted && effective.has(permission.name)

              return (
                <label key={permission.name} className="flex items-start gap-2">
                  <Checkbox
                    className="mt-0.5"
                    checked={isGranted || isImplied}
                    disabled={disabled || isImplied}
                    onCheckedChange={() => onToggle(permission.name)}
                  />
                  <span className="flex flex-col">
                    <span className="font-medium">
                      {permission.name}
                      {isImplied && (
                        <span className="ml-1.5 font-normal text-muted-foreground">included</span>
                      )}
                    </span>
                    <span className="text-muted-foreground">{permission.description}</span>
                    {permission.implies.length > 0 && (
                      <span className="text-muted-foreground">
                        includes {permission.implies.join(", ")}
                      </span>
                    )}
                  </span>
                </label>
              )
            })}
          </div>
        </div>
      ))}
      {sections.length === 0 && <p className="text-muted-foreground">No permissions available.</p>}
    </div>
  )
}
