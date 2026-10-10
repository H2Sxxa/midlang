/** Direction a list query orders its results by. */
export type SortOrder = "asc" | "desc"

/** One choice offered by a filter select. */
export type SelectOption = {
  readonly value: string
  readonly label: string
}
