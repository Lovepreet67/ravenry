import { iamApiClient as apiClient } from "@/lib/api/clients";

import type {
  CreateTenantInput,
  PaginatedList,
  Tenant,
  UpdateTenantInput,
} from "./types";

export interface TenantListParams {
  page_no: number;
  page_size: number;
  is_active?: boolean;
  name?: string;
  slug?: string;
}

export async function listTenants(
  params: TenantListParams,
): Promise<PaginatedList<Tenant>> {
  const response = await apiClient.get<PaginatedList<Tenant>>("/tenants", {
    params,
  });

  return response.data;
}

export async function getTenant(id: string): Promise<Tenant> {
  const response = await apiClient.get<Tenant>(`/tenants/${id}`);
  return response.data;
}

export async function createTenant(input: CreateTenantInput): Promise<Tenant> {
  const response = await apiClient.post<Tenant>("/tenants", input);
  return response.data;
}

export async function updateTenant(
  id: string,
  input: UpdateTenantInput,
): Promise<Tenant> {
  const response = await apiClient.patch<Tenant>(`/tenants/${id}`, input);

  return response.data;
}

export async function deleteTenant(id: string): Promise<void> {
  await apiClient.delete(`/tenants/${id}`);
}
