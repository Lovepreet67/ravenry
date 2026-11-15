import { huginnApiClient as apiClient, inboundApiClient } from "@/lib/api/clients";

import {
  type CursorPage,
  type Delivery,
  type DeliveryAttemptsResponse,
  type EventItem,
  type RetryDeliveryCommand,
  type RetryDeliveryCommandResponse,
} from "./types";

export interface CursorParams {
  limit?: number;
  cursor?: string | null;
}

function toParams(params: CursorParams) {
  return {
    limit: params.limit ?? 10,
    ...(params.cursor ? { cursor: params.cursor } : {}),
  };
}

export async function listEvents(
  tenantId: string,
  params: CursorParams = {},
): Promise<CursorPage<EventItem>> {
  const response = await apiClient.get<CursorPage<EventItem>>(
    `/tenants/${tenantId}/events`,
    { params: toParams(params) },
  );

  return response.data;
}

export async function listDeliveries(
  tenantId: string,
  eventId: string,
  params: CursorParams = {},
): Promise<CursorPage<Delivery>> {
  const response = await apiClient.get<CursorPage<Delivery>>(
    `/tenants/${tenantId}/deliveries`,
    {
      params: {
        ...toParams(params),
        event_id: eventId,
      },
    },
  );

  return response.data;
}

export async function retryDelivery(
  tenantId: string,
  retryCommand: RetryDeliveryCommand
): Promise<RetryDeliveryCommandResponse> {
  const response = await inboundApiClient.post<RetryDeliveryCommandResponse>(
    `/tenants/${tenantId}/retry`, retryCommand
  );
  return response.data;
}

export async function listDeliveryAttempts(
  tenantId: string,
  deliveryId: string,
  params: CursorParams = {},
): Promise<DeliveryAttemptsResponse> {
  const response = await apiClient.get<DeliveryAttemptsResponse>(
    `/tenants/${tenantId}/deliveries/${deliveryId}/attempts`,
    { params: toParams(params) },
  );

  return response.data;
}
