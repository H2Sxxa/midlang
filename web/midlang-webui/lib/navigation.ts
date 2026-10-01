import {
  ChartBarIcon,
  ClockCounterClockwiseIcon,
  ExportIcon,
  GearSixIcon,
  WarningCircleIcon,
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
  readonly label?: string
  readonly items: ReadonlyArray<NavItem>
}

export const navSections: ReadonlyArray<NavSection> = [
  {
    label: undefined,
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
    label: "Diagnostics",
    items: [
      { to: "/issues", label: "Issues", icon: WarningCircleIcon },
      { to: "/coverage", label: "Coverage", icon: ChartBarIcon },
      { to: "/changelog", label: "Changelog", icon: ClockCounterClockwiseIcon },
    ],
  },
  {
    label: "Releases",
    items: [
      { to: "/versions", label: "Versions", icon: ClockCounterClockwiseIcon },
      { to: "/exports", label: "Exports", icon: ExportIcon },
    ],
  },
]

export const footerNavItems: ReadonlyArray<NavItem> = [
  { to: "/settings", label: "Settings", icon: GearSixIcon },
]

export const navItems: ReadonlyArray<NavItem> = [
  ...navSections.flatMap((section) => section.items),
  ...footerNavItems,
]