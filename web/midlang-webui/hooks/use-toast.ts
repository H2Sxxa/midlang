import { toast as sonnerToast } from "sonner"

/**
 * Toast facade: features depend on this module instead of the toast library, so
 * swapping the implementation only touches this file plus the `Toaster` that
 * `components/ui/sonner.tsx` renders.
 */

export type ToastKind = "success" | "info" | "warning" | "error"

export type ToastInput = {
  readonly message: string
  readonly description?: string
  readonly id?: string
}

export type ToastApi = {
  readonly success: (input: ToastInput) => void
  readonly info: (input: ToastInput) => void
  readonly warning: (input: ToastInput) => void
  readonly error: (input: ToastInput) => void
}

type SonnerToast = (
  message: string,
  options: { description?: string; id?: string },
) => string | number

const sonnerKinds: Record<ToastKind, SonnerToast> = {
  success: sonnerToast.success,
  info: sonnerToast.info,
  warning: sonnerToast.warning,
  error: sonnerToast.error,
}

function show(kind: ToastKind, input: ToastInput): void {
  sonnerKinds[kind](input.message, { description: input.description, id: input.id })
}

/** App-wide toast facade, also usable outside React from the query client. */
export const toast: ToastApi = {
  success: (input) => {
    show("success", input)
  },
  info: (input) => {
    show("info", input)
  },
  warning: (input) => {
    show("warning", input)
  },
  error: (input) => {
    show("error", input)
  },
}

/**
 * Returns the toast helpers bound to the app-wide `Toaster`.
 *
 * ```tsx
 * const toast = useToast()
 * toast.success({ message: "Translation saved", description: key })
 * ```
 */
export function useToast(): ToastApi {
  return toast
}
