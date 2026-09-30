import { Outlet, createFileRoute, redirect, useLocation } from "@tanstack/react-router"
import { AppSidebar } from "@/components/app-sidebar"
import { Separator } from "@/components/ui/separator"
import {
  SidebarInset,
  SidebarProvider,
  SidebarTrigger,
} from "@/components/ui/sidebar"
import { APP_TITLE, navItems } from "@/lib/navigation"

export const Route = createFileRoute("/_app")({
  beforeLoad: ({ context, location }) => {
    if (context.auth?.status !== "authenticated") {
      throw redirect({ to: "/login", search: { redirect: location.pathname }, replace: true })
    }
  },
  component: AppLayout,
})

function AppLayout() {
  const pathname = useLocation({ select: (location) => location.pathname })
  const title = [...navItems]
    .sort((left, right) => String(right.to).length - String(left.to).length)
    .find((item) => pathname.startsWith(typeof item.to === "string" ? item.to : ""))
    ?.label ?? APP_TITLE

  return (
    <SidebarProvider>
      <AppSidebar />
      <SidebarInset>
        <header className="flex h-14 shrink-0 items-center gap-2 border-b px-4">
          <SidebarTrigger />
          <Separator orientation="vertical" className="mr-2 h-4 data-vertical:self-center" />
          <h1 className="font-heading text-sm font-medium">{title}</h1>
        </header>
        <div className="flex flex-1 flex-col gap-4 p-4">
          <Outlet />
        </div>
      </SidebarInset>
    </SidebarProvider>
  )
}