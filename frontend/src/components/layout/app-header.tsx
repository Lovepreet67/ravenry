import { Bell, ChevronRight, Command, Search } from "lucide-react";
import { LogOut, ShieldCheck, UserRound } from "lucide-react";
import { useNavigate } from "@tanstack/react-router";
import { useAuth } from "@/features/auth/auth-provider";

export function AppHeader() {
  const { user, mode, logout } = useAuth();
  const navigate = useNavigate();

  function handleLogout() {
    logout();
    void navigate({
      to: mode === "admin" ? "/admin-login" : "/login",
      replace: true,
    });
  }
  return (
    <header className="sticky top-0 z-20 flex h-16 items-center justify-between border-b border-border bg-background/90 px-5 backdrop-blur-xl md:px-8">
      <div className="flex items-center gap-2 text-sm">
        <span className="hidden text-muted-foreground sm:inline">
          Workspace
        </span>
        <ChevronRight
          size={14}
          className="hidden text-muted-foreground sm:block"
        />
        <span className="font-medium">Overview</span>
      </div>

      <div className="flex items-center gap-2">
        <button
          aria-label="Search"
          className="hidden h-9 items-center gap-5 rounded-md border border-border bg-card px-3 text-sm text-muted-foreground transition hover:border-ring/50 sm:flex"
        >
          <span className="flex items-center gap-2">
            <Search size={15} />
            Search
          </span>
          <span className="flex items-center gap-1 rounded border border-border px-1.5 py-0.5 text-[10px]">
            <Command size={10} /> K
          </span>
        </button>

        <button
          aria-label="Notifications"
          className="relative flex size-9 items-center justify-center rounded-md text-muted-foreground transition hover:bg-accent hover:text-foreground"
        >
          <Bell size={18} />
          <span className="absolute right-2 top-2 size-1.5 rounded-full bg-primary" />
        </button>

        <div className="flex items-center gap-3">
          <div className="hidden text-right sm:block">
            <p className="text-sm font-medium">
              {mode === "admin" ? "Administrator" : (user?.username ?? "User")}
            </p>
            <p className="text-xs text-muted-foreground">
              {mode === "admin" ? "Admin access" : user?.email}
            </p>
          </div>

          <div className="flex size-9 items-center justify-center rounded-full border border-border bg-accent text-accent-foreground">
            {mode === "admin" ? (
              <ShieldCheck size={17} />
            ) : (
              <UserRound size={17} />
            )}
          </div>

          <button
            type="button"
            onClick={handleLogout}
            className="flex size-9 items-center justify-center rounded-md text-muted-foreground transition hover:bg-accent hover:text-foreground"
            aria-label="Log out"
            title="Log out"
          >
            <LogOut size={17} />
          </button>
        </div>
      </div>
    </header>
  );
}
