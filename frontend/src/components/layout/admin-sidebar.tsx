import { Link, useRouterState } from "@tanstack/react-router";
import {
  Building2,
  LayoutDashboard,
  Settings,
  ShieldCheck,
  Users,
} from "lucide-react";

const navigation = [
  {
    label: "Dashboard",
    to: "/",
    icon: LayoutDashboard,
  },
  {
    label: "Tenants",
    to: "/tenants",
    icon: Building2,
  },
  {
    label: "Users",
    to: "/users",
    icon: Users,
  },
  {
    label: "Memberships",
    to: "/memberships",
    icon: ShieldCheck,
  },
];

function isActiveRoute(pathname: string, path: string) {
  return pathname === path || pathname.startsWith(`${path}/`);
}

export function AdminSidebar() {
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });

  return (
    <aside className="flex h-screen w-64 shrink-0 flex-col border-r bg-background">
      {/* Brand */}
      <div className="flex h-16 items-center gap-3 border-b px-5">
        <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-primary text-primary-foreground">
          <ShieldCheck size={20} />
        </div>

        <div>
          <h1 className="text-base font-semibold tracking-tight">Ravenry</h1>
          <p className="text-xs text-muted-foreground">
            Platform administration
          </p>
        </div>
      </div>

      {/* Navigation */}
      <nav className="flex-1 space-y-1 overflow-y-auto p-3">
        <p className="mb-3 px-3 pt-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Platform
        </p>

        {navigation.map(({ label, to, icon: Icon }) => {
          const active = isActiveRoute(pathname, to);

          return (
            <Link
              key={to}
              to={to}
              className={[
                "flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm transition-colors",
                active
                  ? "bg-accent font-medium text-accent-foreground"
                  : "text-muted-foreground hover:bg-accent/60 hover:text-foreground",
              ].join(" ")}
            >
              <Icon size={18} />
              <span>{label}</span>
            </Link>
          );
        })}
      </nav>

      {/* Footer */}
      <div className="border-t p-3">
        <Link
          to="/settings"
          className={[
            "flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm transition-colors",
            isActiveRoute(pathname, "/settings")
              ? "bg-accent font-medium text-accent-foreground"
              : "text-muted-foreground hover:bg-accent/60 hover:text-foreground",
          ].join(" ")}
        >
          <Settings size={18} />
          <span>Settings</span>
        </Link>

        <div className="mt-3 rounded-lg bg-muted/60 px-3 py-2">
          <p className="text-xs font-medium">Admin workspace</p>
          <p className="mt-0.5 text-xs text-muted-foreground">Global access</p>
        </div>
      </div>
    </aside>
  );
}
