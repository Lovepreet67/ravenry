import { Link, useRouterState } from "@tanstack/react-router";
import {
  ArrowLeft,
  Building2,
  LayoutDashboard,
  Network,
  Activity,
  Settings,
} from "lucide-react";

import { useAuth } from "@/features/auth/auth-provider";
import { useCurrentTenant } from "@/features/tenants/tenant-provider";
import { TenantSwitcher } from "@/features/tenants/components/tenant-switcher";

const navigation = [
  {
    label: "Overview",
    to: "/tenants/$tenantId",
    icon: LayoutDashboard,
    isExact: true,
    roles: [],
  },
  {
    label: "Destinations",
    to: "/tenants/$tenantId/destinations",
    icon: Network,
    roles:null,
  },
  {
    label: "Topics",
    to: "/tenants/$tenantId/topics",
    icon: Building2,
    roles:null
  },
  {
    label: "Events",
    to: "/tenants/$tenantId/events",
    icon: Activity,
    roles:undefined
  },
];

function isActiveRoute(pathname: string, path: string, isExact = false) {
  return isExact
    ? pathname === path
    : pathname === path || pathname.startsWith(`${path}/`);
}

export function TenantSidebar() {
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });

  const { mode } = useAuth();
  const { currentTenant } = useCurrentTenant();

  const isAdmin = mode === "admin";

  // Admins may not have a currentTenant in membership context.
  // In that case, derive the tenant ID from the current URL.
  const tenantId =
    currentTenant?.tenant_id ??
    pathname.match(/^\/tenants\/([^/]+)/)?.[1] ??
    null;

  const tenantName = currentTenant?.tenant_name ?? "Tenant workspace";

  return (
    <aside className="flex h-screen w-64 shrink-0 flex-col border-r bg-background">
      {/* Brand */}
      <div className="flex h-16 items-center gap-3 border-b px-5">
        <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-primary text-primary-foreground">
          <Building2 size={20} />
        </div>

        <div className="min-w-0">
          <h1 className="text-base font-semibold tracking-tight">Ravenry</h1>
          <p className="text-xs text-muted-foreground">Tenant workspace</p>
        </div>
      </div>

      {/* Tenant selection */}
      <div className="border-b p-3">
        <p className="mb-2 px-1 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Current tenant
        </p>

        <TenantSwitcher />

        {tenantId && (
          <div className="mt-2 flex items-center gap-2 rounded-lg bg-muted/60 px-3 py-2">
            <Building2 size={16} className="shrink-0 text-muted-foreground" />

            <div className="min-w-0">
              <p className="truncate text-sm font-medium">{tenantName}</p>

              {currentTenant?.roles.length && (
                <p className="truncate text-xs text-muted-foreground">
                  {currentTenant.roles.join(", ")}
                </p>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Navigation */}
      <nav className="flex-1 space-y-1 overflow-y-auto p-3">
        <p className="mb-3 px-3 pt-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Workspace
        </p>

        {tenantId ? (
          navigation
            .filter(
              (item) =>
                !item.roles || item.roles.length==0||
                item?.roles.find((role) =>
                  currentTenant?.tenant_slug?.includes(role),
                ),
            )
            .map(({ label, to, icon: Icon, isExact }) => {
              // Convert the route template to its concrete URL for active matching.
              const path = to.replace("$tenantId", tenantId);
              const active = isActiveRoute(pathname, path, isExact);

              return (
                <Link
                  key={to}
                  to={to}
                  params={{ tenantId }}
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
            })
        ) : (
          <p className="px-3 py-2 text-sm text-muted-foreground">
            Select a tenant to view its resources.
          </p>
        )}
      </nav>

      {/* Footer */}
      <div className="border-t p-3">
        {isAdmin && (
          <Link
            to="/tenants"
            className="mb-2 flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm text-muted-foreground transition-colors hover:bg-accent/60 hover:text-foreground"
          >
            <ArrowLeft size={18} />
            <span>Back to platform</span>
          </Link>
        )}

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
      </div>
    </aside>
  );
}
