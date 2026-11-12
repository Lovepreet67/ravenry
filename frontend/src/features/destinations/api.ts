import { iamApiClient as apiClient } from "@/lib/api/clients";

import type {
  CreateDestinationInput,
  Destination,
  DestinationListParams,
  PaginatedList,
  UpdateDestinationInput,
} from "./types";

const basePath = (tenantId: string) => `/tenants/${tenantId}/destinations`;

export async function listDestinations(
  tenantId: string,
  params: DestinationListParams,
): Promise<PaginatedList<Destination>> {
  const response = await apiClient.get<PaginatedList<Destination>>(
    basePath(tenantId),
    { params },
  );

  return response.data;
}

export async function createDestination(
  tenantId: string,
  input: CreateDestinationInput,
): Promise<Destination> {
  const response = await apiClient.post<Destination>(basePath(tenantId), input);

  return response.data;
}

export async function getDestination(
  tenantId: string,
  destinationId: string,
): Promise<Destination> {
  const response = await apiClient.get<Destination>(
    `${basePath(tenantId)}/${destinationId}`,
  );

  return response.data;
}

export async function updateDestination(
  tenantId: string,
  destinationId: string,
  input: UpdateDestinationInput,
): Promise<Destination> {
  const response = await apiClient.patch<Destination>(
    `${basePath(tenantId)}/${destinationId}`,
    input,
  );

  return response.data;
}
