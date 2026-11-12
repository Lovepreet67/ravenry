import { iamApiClient as apiClient } from "@/lib/api/clients";

import type {
  CreateUserInput,
  PaginatedList,
  User,
  UserListParams,
  UpdateUserInput,
} from "./types";

export async function listUsers(
  params: UserListParams,
): Promise<PaginatedList<User>> {
  const response = await apiClient.get<PaginatedList<User>>("/users", {
    params,
  });

  return response.data;
}

export async function createUser(input: CreateUserInput): Promise<User> {
  const response = await apiClient.post<User>("/users", input);
  return response.data;
}

export async function getUser(id: string): Promise<User> {
  const response = await apiClient.get<User>(`/users/${id}`);
  return response.data;
}

export async function updateUser(
  id: string,
  input: UpdateUserInput,
): Promise<User> {
  const response = await apiClient.patch<User>(`/users/${id}`, input);
  return response.data;
}

export async function deleteUser(id: string): Promise<string> {
  const response = await apiClient.delete<string>(`/users/${id}`);
  return response.data;
}
