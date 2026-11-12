export interface Tenant {
  id: string;
  name: string;
  slug: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface TenantMembershipSummary {
  tenant_id: string;
  tenant_name: string;
  tenant_slug: string;
  roles: string[];
}

export interface CreateTenantInput {
  name: string;
  slug: string;
  default_role_slug?: string;
}

export interface UpdateTenantInput {
  id?: string;
  name?: string;
  is_active?: boolean;
}

export interface PageInfo {
  page_size: number;
  page_no: number;
  total_rows: number;
}

export interface PaginatedList<T> {
  list: T[];
  page_info: PageInfo;
}
