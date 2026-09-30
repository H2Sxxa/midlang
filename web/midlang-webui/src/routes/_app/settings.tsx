import { createFileRoute } from "@tanstack/react-router"
import { MonitorIcon, MoonIcon, SunIcon, type Icon } from "@phosphor-icons/react"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Label } from "@/components/ui/label"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { themeOptions, useTheme, type ThemePreference } from "@/hooks/use-theme"

const themeIcons: Record<ThemePreference, Icon> = {
  light: SunIcon,
  dark: MoonIcon,
  system: MonitorIcon,
}

export const Route = createFileRoute("/_app/settings")({
  component: SettingsPage,
})

function SettingsPage() {
  const { preference, resolvedTheme, setPreference } = useTheme()

  const handlePreferenceChange = (value: ThemePreference): void => {
    setPreference(value)
  }

  return (
    <Card className="max-w-2xl">
      <CardHeader>
        <CardTitle>Settings</CardTitle>
        <CardDescription>Console and workspace preferences.</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <Label id="theme-preference-label">Theme</Label>
        <Tabs value={preference} onValueChange={handlePreferenceChange}>
          <TabsList aria-labelledby="theme-preference-label">
            {themeOptions.map((option) => {
              const OptionIcon = themeIcons[option.value]

              return (
                <TabsTrigger key={option.value} value={option.value}>
                  <OptionIcon />
                  {option.label}
                </TabsTrigger>
              )
            })}
          </TabsList>
        </Tabs>
        <p className="text-muted-foreground">
          System follows the operating system setting. The console is currently rendering the{" "}
          {resolvedTheme} theme.
        </p>
      </CardContent>
    </Card>
  )
}

