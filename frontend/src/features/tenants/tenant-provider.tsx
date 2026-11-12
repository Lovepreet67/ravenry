import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";

import { useAuth } from "@/features/auth/auth-provider";
import type { MembershipDetails } from "@/features/auth/types";
import type { TenantMembershipSummary } from "./types";

const STORAGE_KEY = "ravenry.current_tenant_id";

interface TenantContextValue {
  memberships: MembershipDetails[];
  currentTenant: {
    tenant_id: string | null;
    tenant_name: string | null;
    tenant_slug: string | null;
    roles: string[];
  } | null;
  tenantMembershipSummaries: TenantMembershipSummary[];
  setCurrentTenant: (tenantId: string) => void;
}

const TenantContext = createContext<TenantContextValue | null>(null);

export function TenantProvider({ children }: { children: ReactNode }) {
  const { user, mode, isLoading } = useAuth();
  const [selectedTenantId, setSelectedTenantId] = useState<string | null>(null);

  const memberships = mode === "user" ? (user?.memberships ?? []) : [];

  useEffect(() => {
    if (isLoading) return;

    if (mode !== "user") {
      sessionStorage.removeItem(STORAGE_KEY);
      setSelectedTenantId(null);
      return;
    }

    const storedId = sessionStorage.getItem(STORAGE_KEY);
    const validStoredMembership = memberships.find(
      (membership) => membership.tenant_id === storedId,
    );
    const nextTenantId =
      validStoredMembership?.tenant_id ?? memberships[0]?.tenant_id ?? null;

    setSelectedTenantId(nextTenantId);

    if (nextTenantId) {
      sessionStorage.setItem(STORAGE_KEY, nextTenantId);
    } else {
      sessionStorage.removeItem(STORAGE_KEY);
    }
  }, [isLoading, mode, memberships]);

  const setCurrentTenant = useCallback(
    (tenantId: string) => {
      const membership = memberships.find(
        (item) => item.tenant_id === tenantId,
      );

      if (!membership) {
        throw new Error("You don't have access to this tenant.");
      }

      sessionStorage.setItem(STORAGE_KEY, tenantId);
      setSelectedTenantId(tenantId);
    },
    [memberships],
  );
  const currentTenantMemberships = memberships.filter(
    (membership) => membership.tenant_id === selectedTenantId,
  );
  const currentTenantFirstMembership = currentTenantMemberships?.[0];
  const currentTenantRole =
    currentTenantMemberships.map((m) => m.role_slug) ?? null;
  const currentTenant = {
    ...(currentTenantFirstMembership || {}),
    roles: currentTenantRole,
  };
  const tenantMembershipSummaries: TenantMembershipSummary[] = Object.values(
    memberships.reduce<Record<string, TenantMembershipSummary>>(
      (prev, curr) => {
        if (prev[curr.tenant_id]) {
          prev[curr.tenant_id].roles.push(curr.role_slug);
        } else {
          prev[curr.tenant_id] = {
            tenant_id: curr.tenant_id,
            tenant_name: curr.tenant_name,
            tenant_slug: curr.tenant_slug,
            roles: [],
          };
        }
        return prev;
      },
      {},
    ),
  );
  return (
    <TenantContext.Provider
      value={{
        memberships,
        currentTenant,
        tenantMembershipSummaries,
        setCurrentTenant,
      }}
    >
      {children}
    </TenantContext.Provider>
  );
}

export function useCurrentTenant(): TenantContextValue {
  const context = useContext(TenantContext);

  if (!context) {
    throw new Error("useCurrentTenant must be used within TenantProvider");
  }

  return context;
}
