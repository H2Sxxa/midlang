import {
  GearSixIcon,
  TranslateIcon,
  SquaresFourIcon,
  type Icon,
} from "@phosphor-icons/react"
import type { LinkProps } from "@tanstack/react-router"

export const APP_TITLE = "MidLang"

export type NavItem = {
  readonly to: LinkProps["to"]
  readonly label: string
  readonly icon: Icon
}

export type NavSection = {
  readonly label: string
  readonly items: ReadonlyArray<NavItem>
}

export const navSections: ReadonlyArray<NavSection> = [
  {
    label: "Platform",
    items: [{ to: "/", label: "Overview", icon: SquaresFourIcon }],
  },
  {
    label: "Translation",
    items: [
      { to: "/translations", label: "Translation List", icon: TranslateIcon },
      { to: "/translations/new", label: "Add Translation", icon: TranslateIcon },
    ],
  },
  {
    label: "Workspace",
    items: [{ to: "/settings", label: "Settings", icon: GearSixIcon }],
  },
]

export const navItems: ReadonlyArray<NavItem> = navSections.flatMap((section) => section.items)