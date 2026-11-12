import { iamApiClient as apiClient } from "@/lib/api/clients";
import type { LoginRequest, LoginResponse, UserInfo } from "./types";

export async function loginRequest(
  input: LoginRequest,
): Promise<LoginResponse> {
  const response = await apiClient.post<LoginResponse>("/auth/login", input);

  return response.data;
}

export async function getCurrentUser(): Promise<UserInfo> {
  const response = await apiClient.get<UserInfo>("/auth/me");
  return response.data;
}
