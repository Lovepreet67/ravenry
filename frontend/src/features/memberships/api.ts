import { iamApiClient as apiClient } from "@/lib/api/clients";

import type {
  MembershipDetails,
  UpsertMembershipInput,
  UserMemberships,
} from "./types";

export async function getUserMemberships(
  userId: string,
): Promise<UserMemberships> {
  const response = await apiClient.get<UserMemberships>(
    `/users/${userId}/memberships`,
  );

  return response.data;
}

export async function upsertMembership(
  tenantId: string,
  input: UpsertMembershipInput,
): Promise<MembershipDetails> {
  const response = await apiClient.post<MembershipDetails>(
    `/tenants/${tenantId}/memberships`,
    input,
  );

  return response.data;
}
