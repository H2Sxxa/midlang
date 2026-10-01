import {
  GearSixIcon,
  TranslateIcon,
  SquaresFourIcon,
  type Icon,
} from "@phosphor-icons/react"
import type { LinkProps } from "@tanstack/react-router"
import type { Permission } from "@/hooks/use-auth"

export const APP_TITLE = "MidLang"

export type NavItem = {
  readonly to: LinkProps["to"]
  readonly label: string
  readonly icon: Icon
  readonly permission?: Permission
}

export type NavSection = {
  readonly label: string
  readonly items: ReadonlyArray<NavItem>
}

export const navSections: ReadonlyArray<NavSection> = [
  {
    label: "Platform",
    items: [{ to: "/", label: "Overview", icon: SquaresFourIcon, permission: "translation:list" }],
  },
  {
    label: "Translation",
    items: [
      { to: "/translations", label: "Translation List", icon: TranslateIcon, permission: "translation:list" },
      { to: "/translations/new", label: "Add Translation", icon: TranslateIcon, permission: "translation:write" },
    ],
  },
  {
    label: "Workspace",
    items: [{ to: "/settings", label: "Settings", icon: GearSixIcon }],
  },
]

export const navItems: ReadonlyArray<NavItem> = navSections.flatMap((section) => section.items)