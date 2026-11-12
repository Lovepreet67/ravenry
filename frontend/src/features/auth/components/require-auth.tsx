import { useEffect, type ReactNode } from "react";
import { useNavigate } from "@tanstack/react-router";

import { useAuth, type AuthMode } from "@/features/auth/auth-provider";
import { useCurrentTenant } from "@/features/tenants/tenant-provider";

interface RequireAuthProps {
  children: ReactNode;
  requiredMode: Exclude<AuthMode, null> | "any";
  requiredRoles?: string[];
}

export function RequireAuth({
  children,
  requiredMode,
  requiredRoles,
}: RequireAuthProps) {
  const { mode, isLoading } = useAuth();
  const { currentTenant } = useCurrentTenant();
  const navigate = useNavigate();

  const authenticated =
    mode !== null && (requiredMode === "any" || mode === requiredMode);

  const hasRequiredRole =
    !requiredRoles?.length ||
    (mode === "admin" && requiredMode !== "any") ||
    currentTenant?.roles.find((role) => requiredRoles.includes(role));

  useEffect(() => {
    if (isLoading || authenticated) return;

    const destination = requiredMode === "admin" ? "/admin-login" : "/login";

    void navigate({ to: destination, replace: true });
  }, [authenticated, isLoading, navigate, requiredMode]);

  if (isLoading || !authenticated) {
    return (
      <div className="flex min-h-64 items-center justify-center">
        <div className="size-6 animate-spin rounded-full border-2 border-border border-t-primary" />
      </div>
    );
  }

  if (!hasRequiredRole) {
    return (
      <div className="flex min-h-64 flex-col items-center justify-center gap-2 px-6 text-center">
        <h2 className="text-lg font-semibold">Access denied</h2>
        <p className="max-w-md text-sm text-muted-foreground">
          You don't have permission to access this resource. Contact your tenant
          administrator if you believe this is a mistake.
        </p>
      </div>
    );
  }

  return <>{children}</>;
}
