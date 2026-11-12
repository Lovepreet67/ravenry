import { iamApiClient as apiClient } from "@/lib/api/clients";

import type { Role } from "./types";

export async function listRoles(): Promise<Role[]> {
  const response = await apiClient.get<Role[]>("/roles");
  return response.data;
}
