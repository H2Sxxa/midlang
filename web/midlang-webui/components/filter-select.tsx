import { Label } from "@/components/ui/label"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import type { SelectOption } from "@/lib/query"

type FilterSelectProps = {
  readonly id: string
  readonly label: string
  readonly placeholder: string
  readonly value: string
  readonly options: ReadonlyArray<SelectOption>
  readonly onChange: (value: string) => void
}

/**
 * Labelled select used by every list filter, so filter rows look and behave the
 * same on every page.
 */
export function FilterSelect({
  id,
  label,
  placeholder,
  value,
  options,
  onChange,
}: FilterSelectProps) {
  return (
    <div className="flex flex-col gap-1.5">
      <Label htmlFor={id}>{label}</Label>
      <Select
        items={options}
        value={value}
        onValueChange={(next) => {
          if (next !== null) {
            onChange(next)
          }
        }}
      >
        <SelectTrigger id={id} className="w-44">
          <SelectValue placeholder={placeholder} />
        </SelectTrigger>
        <SelectContent>
          {options.map((option) => (
            <SelectItem key={option.value} value={option.value}>
              {option.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  )
}
