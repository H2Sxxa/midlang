import { useState } from "react"
import { createFileRoute } from "@tanstack/react-router"
import {
  CaretDownIcon,
  ListIcon,
  MonitorIcon,
  MoonIcon,
  SunIcon,
  type Icon,
} from "@phosphor-icons/react"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Label } from "@/components/ui/label"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar"
import { themeOptions, useTheme, type ThemePreference } from "@/hooks/use-theme"

const themeIcons: Record<ThemePreference, Icon> = {
  light: SunIcon,
  dark: MoonIcon,
  system: MonitorIcon,
}

type SettingsSection = "appearance" | "workspace"

type SettingsSectionDefinition = {
  readonly id: SettingsSection
  readonly label: string
  readonly icon: Icon
}

const settingsSections: readonly SettingsSectionDefinition[] = [
  { id: "appearance", label: "Appearance", icon: SunIcon },
  { id: "workspace", label: "Workspace", icon: MonitorIcon },
]

export const Route = createFileRoute("/_app/settings")({
  component: SettingsPage,
})

function SettingsPage() {
  const [activeSection, setActiveSection] = useState<SettingsSection>("appearance")

  return (
    <div className="-m-4 flex min-h-[calc(100vh-3.5rem)] flex-col md:flex-row">
      <SettingsNavigation activeSection={activeSection} onSectionChange={setActiveSection} />

      <main className="min-w-0 flex-1 px-5 py-6 md:px-8 md:py-8">
        <div className="mx-auto w-full max-w-4xl">
          <SettingsSectionContent activeSection={activeSection} />
        </div>
      </main>
    </div>
  )
}

type SettingsNavigationProps = {
  readonly activeSection: SettingsSection
  readonly onSectionChange: (section: SettingsSection) => void
}

function SettingsNavigation({ activeSection, onSectionChange }: SettingsNavigationProps) {
  const activeSectionLabel = settingsSections.find((section) => section.id === activeSection)?.label

  return (
    <>
      <Sidebar collapsible="none" className="hidden h-full w-56 shrink-0 border-r md:flex">
        <SidebarHeader className="border-b px-4 py-5">
          <h2 className="font-heading text-base font-semibold tracking-tight">Settings</h2>
          <p className="text-xs text-muted-foreground">Manage your workspace.</p>
        </SidebarHeader>
        <SidebarContent>
          <SidebarGroup>
            <SidebarGroupLabel>Preferences</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu aria-label="Settings sections">
                {settingsSections.map((section) => {
                  const SectionIcon = section.icon

                  return (
                    <SidebarMenuItem key={section.id}>
                      <SidebarMenuButton
                        isActive={activeSection === section.id}
                        onClick={() => onSectionChange(section.id)}
                      >
                        <SectionIcon />
                        <span>{section.label}</span>
                      </SidebarMenuButton>
                    </SidebarMenuItem>
                  )
                })}
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
        </SidebarContent>
      </Sidebar>

      <div className="border-b p-3 md:hidden">
        <DropdownMenu>
          <DropdownMenuTrigger
            render={
              <Button variant="outline" className="w-full justify-between">
                <span className="flex items-center gap-2">
                  <ListIcon />
                  {activeSectionLabel}
                </span>
                <CaretDownIcon />
              </Button>
            }
          />
          <DropdownMenuContent align="start" className="w-[calc(100vw-1.5rem)]">
            {settingsSections.map((section) => {
              const SectionIcon = section.icon

              return (
                <DropdownMenuItem key={section.id} onClick={() => onSectionChange(section.id)}>
                  <SectionIcon />
                  {section.label}
                </DropdownMenuItem>
              )
            })}
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </>
  )
}

function SettingsSectionContent({ activeSection }: { readonly activeSection: SettingsSection }) {
  return activeSection === "appearance" ? <AppearanceSettings /> : <WorkspaceSettings />
}

function AppearanceSettings() {
  const { preference, resolvedTheme, setPreference } = useTheme()
  const CurrentIcon = themeIcons[preference]

  return (
    <section aria-labelledby="appearance-heading" className="space-y-6">
      <div className="space-y-1">
        <h1 id="appearance-heading" className="font-heading text-xl font-semibold tracking-tight">
          Appearance
        </h1>
        <p className="text-sm text-muted-foreground">Customize how Midlang looks on this device.</p>
      </div>
      <Card>
        <CardHeader className="border-b">
          <CardTitle>Theme</CardTitle>
          <CardDescription>Choose a theme or follow your operating system preference.</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4 p-5 sm:flex-row sm:items-center sm:justify-between">
          <div className="flex items-start gap-3">
            <div className="flex size-8 shrink-0 items-center justify-center border bg-muted text-muted-foreground">
              <CurrentIcon />
            </div>
            <div className="space-y-1">
              <Label id="theme-preference-label" className="text-sm font-medium">Color scheme</Label>
              <p className="text-xs text-muted-foreground">
                Currently using <span className="font-medium text-foreground">{resolvedTheme}</span> mode.
              </p>
            </div>
          </div>
          <Tabs value={preference} onValueChange={setPreference} className="w-full sm:w-auto">
            <TabsList aria-labelledby="theme-preference-label" className="grid h-9 w-full grid-cols-3 sm:w-auto">
              {themeOptions.map((option) => {
                const OptionIcon = themeIcons[option.value]

                return (
                  <TabsTrigger key={option.value} value={option.value} className="px-3">
                    <OptionIcon />
                    <span>{option.label}</span>
                  </TabsTrigger>
                )
              })}
            </TabsList>
          </Tabs>
        </CardContent>
      </Card>
    </section>
  )
}

function WorkspaceSettings() {
  return (
    <section aria-labelledby="workspace-heading" className="space-y-6">
      <div className="space-y-1">
        <h1 id="workspace-heading" className="font-heading text-xl font-semibold tracking-tight">Workspace</h1>
        <p className="text-sm text-muted-foreground">Settings for the current translation workspace.</p>
      </div>
      <Card>
        <CardContent className="flex items-center justify-between gap-4 p-5">
          <div className="space-y-1">
            <p className="text-sm font-medium">Translation preferences</p>
            <p className="text-xs text-muted-foreground">Workspace-specific options will appear here as they become available.</p>
          </div>
          <span className="shrink-0 border px-2 py-1 text-[11px] text-muted-foreground">Coming soon</span>
        </CardContent>
      </Card>
    </section>
  )
}

