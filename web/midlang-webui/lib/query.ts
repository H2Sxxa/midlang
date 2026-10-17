/** Direction a list query orders its results by. */
export type SortOrder = "asc" | "desc"

/** One choice offered by a filter select. */
export type SelectOption = {
  readonly value: string
  readonly label: string
}

/** Sort directions a list accepts, in the order a control should offer them. */
export const SORT_ORDERS: ReadonlyArray<SortOrder> = ["asc", "desc"]

/**
 * Cursor of a list's first page.
 *
 * A list pages by opaque cursor, so the page on screen is a single `cursor`
 * search param: empty (this value) on the first page, otherwise the `next` token
 * of the page that led to it. A cursor names only a position: the filters and
 * order travel as their own search params, so a link stays short and shareable,
 * and walking back is the reader's own trail of cursors rather than something
 * the endpoint tracks.
 */
export const FIRST_PAGE = ""

/** Reads a string search param; anything that is not a string reads as empty. */
export function parseText(value: unknown): string {
  return typeof value === "string" ? value : ""
}

/** Reads a search param that must be one of `options`. */
export function parseChoice<TValue extends string>(
  value: unknown,
  options: ReadonlyArray<TValue>,
  fallback: TValue,
): TValue {
  if (typeof value !== "string") {
    return fallback
  }
  return options.find((option) => option === value) ?? fallback
}
