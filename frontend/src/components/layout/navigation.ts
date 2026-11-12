import {
  LayoutDashboard,
  Building2,
  Users,
  ShieldCheck,
  Workflow,
  type LucideIcon,
} from "lucide-react";

export interface NavigationItem {
  label: string;
  icon: LucideIcon;
  scope: "tenant" | "admin";
  roles?: string[];
  getPath: (tenantId: string | null) => string;
}

export const navigation: NavigationItem[] = [
  {
    label: "Overview",
    icon: LayoutDashboard,
    scope: "tenant",
    getPath: (id) => `/tenants/${id}`,
  },
  {
    label: "Destinations",
    icon: Workflow,
    scope: "tenant",
    roles: ["tenant_admin", "editor", "viewer"],
    getPath: (id) => `/tenants/${id}/destinations`,
  },
  {
    label: "Topics",
    icon: Workflow,
    scope: "tenant",
    roles: ["tenant_admin", "editor", "viewer"],
    getPath: (id) => `/tenants/${id}/topics`,
  },
  {
    label: "Memberships",
    icon: ShieldCheck,
    scope: "tenant",
    roles: ["tenant_admin"],
    getPath: (id) => `/tenants/${id}/memberships`,
  },

  // Only shown in the admin session
  {
    label: "All Tenants",
    icon: Building2,
    scope: "admin",
    getPath: () => "/tenants",
  },
  {
    label: "Global Dashboard",
    icon: LayoutDashboard,
    scope: "admin",
    getPath: () => "/admin/dashboard",
  },
  {
    label: "All Users",
    icon: Users,
    scope: "admin",
    getPath: () => "/users",
  },
];
