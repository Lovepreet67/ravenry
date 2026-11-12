import { Outlet } from "@tanstack/react-router";
import { AppHeader } from "@/components/layout/app-header";
import { AppSidebar } from "@/components/layout/app-sidebar";
import { useAuth } from "@/features/auth/auth-provider";

export function AppLayout() {
  const authContext = useAuth();
  return (
    <div className="flex min-h-screen bg-background">
      {authContext.isAuthenticated && (
        <div className="hidden md:flex">
          <AppSidebar />
        </div>
      )}

      <div className="flex min-w-0 flex-1 flex-col">
        <AppHeader />
        <main className="flex-1 px-5 py-7 md:px-8 md:py-9">
          <div className="mx-auto w-full max-w-[1440px]">
            <Outlet />
          </div>
        </main>

        <footer className="flex flex-col gap-1 border-t border-border px-5 py-4 text-[11px] text-muted-foreground sm:flex-row sm:items-center sm:justify-between md:px-8">
          <span>Ravenry Systems Console</span>
          <span>Administrative workspace · Development</span>
        </footer>
      </div>
    </div>
  );
}
