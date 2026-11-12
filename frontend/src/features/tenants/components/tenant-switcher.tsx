import { Building2, ChevronDown } from "lucide-react";

import { useAuth } from "@/features/auth/auth-provider";
import { useCurrentTenant } from "@/features/tenants/tenant-provider";

export function TenantSwitcher() {
  const { mode } = useAuth();
  const {
    memberships,
    currentTenant,
    setCurrentTenant,
    tenantMembershipSummaries,
  } = useCurrentTenant();

  // Administrators manage tenants rather than selecting a membership.
  if (mode !== "user") return null;

  if (memberships.length === 0) {
    return (
      <div className="rounded-lg border border-border bg-background/60 p-3">
        <div className="flex items-center gap-2 text-sm">
          <Building2 size={16} className="text-muted-foreground" />
          <span className="font-medium">No tenant access</span>
        </div>
        <p className="mt-1 text-xs text-muted-foreground">
          Your account has no tenant memberships yet.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-2">
      <p className="px-1 text-[10px] font-semibold uppercase tracking-[0.18em] text-muted-foreground">
        Current workspace
      </p>

      <label className="relative flex items-center gap-3 rounded-lg border border-border bg-background px-3 py-2.5 transition hover:bg-accent/40">
        <div className="flex size-9 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary">
          <Building2 size={17} />
        </div>

        <div className="min-w-0 flex-1">
          <span className="block truncate text-sm font-medium">
            {currentTenant?.tenant_name ?? "Select tenant"}
          </span>
          <span className="block truncate text-xs text-muted-foreground">
            {currentTenant
              ? `${currentTenant.tenant_slug}`
              : "Choose a workspace"}
          </span>
        </div>

        <ChevronDown size={15} className="shrink-0 text-muted-foreground" />

        <select
          aria-label="Select current tenant"
          value={currentTenant?.tenant_id ?? ""}
          onChange={(event) => setCurrentTenant(event.target.value)}
          className="absolute inset-0 h-full w-full cursor-pointer opacity-0"
        >
          {tenantMembershipSummaries.map((membership) => (
            <option key={membership.tenant_id} value={membership.tenant_id}>
              {membership.tenant_name}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}
