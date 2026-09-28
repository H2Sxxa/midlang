import { GearSixIcon, SquaresFourIcon, type Icon } from "@phosphor-icons/react"
import type { LinkProps } from "@tanstack/react-router"

export const APP_TITLE = "MidLang"

export type NavItem = {
  readonly to: LinkProps["to"]
  readonly label: string
  readonly icon: Icon
}

export const navItems: ReadonlyArray<NavItem> = [
  { to: "/", label: "Overview", icon: SquaresFourIcon },
  { to: "/settings", label: "Settings", icon: GearSixIcon },
]