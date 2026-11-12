import { iamApiClient as apiClient } from "@/lib/api/clients";

import type {
  CreateTopicInput,
  Topic,
  TopicListParams,
  PaginatedList,
  UpdateTopicInput,
} from "./types";

const basePath = (tenantId: string) => `/tenants/${tenantId}/topics`;

export async function listTopics(
  tenantId: string,
  params: TopicListParams,
): Promise<PaginatedList<Topic>> {
  const response = await apiClient.get<PaginatedList<Topic>>(
    basePath(tenantId),
    { params },
  );

  return response.data;
}

export async function createTopic(
  tenantId: string,
  input: CreateTopicInput,
): Promise<Topic> {
  const response = await apiClient.post<Topic>(basePath(tenantId), input);

  return response.data;
}

export async function getTopic(
  tenantId: string,
  topicId: string,
): Promise<Topic> {
  const response = await apiClient.get<Topic>(
    `${basePath(tenantId)}/${topicId}`,
  );

  return response.data;
}

export async function updateTopic(
  tenantId: string,
  topicId: string,
  input: UpdateTopicInput,
): Promise<Topic> {
  const response = await apiClient.patch<Topic>(
    `${basePath(tenantId)}/${topicId}`,
    input,
  );

  return response.data;
}
