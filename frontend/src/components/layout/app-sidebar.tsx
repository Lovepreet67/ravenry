import { useRouterState } from "@tanstack/react-router";

import { useAuth } from "@/features/auth/auth-provider";

import { AdminSidebar } from "./admin-sidebar";
import { TenantSidebar } from "./tenant-sidebar";

const ADMIN_GLOBAL_PATHS = [
  "/",
  "/dashboard",
  "/admin/dashboard",
  "/tenants",
  "/tenants/new",
  "/users",
  "/memberships",
];

function isAdminGlobalRoute(pathname: string): boolean {
  return (
    ADMIN_GLOBAL_PATHS.includes(pathname) ||
    pathname.startsWith("/users/") ||
    pathname.startsWith("/memberships/")
  );
}

function isTenantRoute(pathname: string): boolean {
  // /tenants is the global tenant listing.
  // /tenants/new is the global create-tenant page.
  if (pathname === "/tenants" || pathname === "/tenants/new") {
    return false;
  }

  // /tenants/:tenantId and its nested routes.
  return /^\/tenants\/[^/]+(?:\/.*)?$/.test(pathname);
}

export function AppSidebar() {
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });

  const { mode } = useAuth();

  const isAdmin = mode === "admin";
  const isTenantContext = isTenantRoute(pathname);

  // Normal users never access the platform-wide admin sidebar.
  if (!isAdmin) {
    return <TenantSidebar />;
  }

  // Admins see tenant-specific navigation when inside a tenant.
  if (isTenantContext) {
    return <TenantSidebar />;
  }

  // Admins see platform-wide navigation everywhere else.
  if (isAdminGlobalRoute(pathname)) {
    return <AdminSidebar />;
  }

  // Use tenant navigation as the fallback for other routes.
  return <TenantSidebar />;
}
