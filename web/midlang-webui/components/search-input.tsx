import { MagnifyingGlassIcon, XIcon } from "@phosphor-icons/react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"

type SearchInputProps = {
  readonly id: string
  readonly label: string
  readonly placeholder: string
  readonly value: string
  readonly onChange: (value: string) => void
}

/**
 * Search box used by every list toolbar. Typing filters once it pauses, so
 * there is no search button; the clear control drops the term immediately.
 */
export function SearchInput({ id, label, placeholder, value, onChange }: SearchInputProps) {
  return (
    <div className="relative flex-1">
      <MagnifyingGlassIcon className="absolute top-2 left-2.5 size-4 text-muted-foreground" />
      <Input
        id={id}
        className="pr-9 pl-8"
        placeholder={placeholder}
        aria-label={label}
        value={value}
        onChange={(event) => onChange(event.target.value)}
      />
      {value !== "" && (
        <Button
          size="icon-xs"
          variant="ghost"
          className="absolute top-1 right-1"
          aria-label={`Clear ${label}`}
          onClick={() => onChange("")}
        >
          <XIcon />
        </Button>
      )}
    </div>
  )
}
