const ACCESS_TOKEN_KEY = "ravenry.access_token";
const REFRESH_TOKEN_KEY = "ravenry.refresh_token";

const AUTH_MODE_KEY = "ravenry.auth_mode";

export type StoredAuthMode = "user" | "admin";

export function getAuthMode(): StoredAuthMode | null {
  const mode = localStorage.getItem(AUTH_MODE_KEY);

  return mode === "user" || mode === "admin" ? mode : null;
}

export function setAuthMode(mode: StoredAuthMode): void {
  localStorage.setItem(AUTH_MODE_KEY, mode);
}

export function clearAuthMode(): void {
  localStorage.removeItem(AUTH_MODE_KEY);
}

export function getAccessToken(): string | null {
  return localStorage.getItem(ACCESS_TOKEN_KEY);
}

export function setAccessToken(token: string): void {
  localStorage.setItem(ACCESS_TOKEN_KEY, token);
}

export function getRefreshToken(): string | null {
  return localStorage.getItem(REFRESH_TOKEN_KEY);
}

export function setRefreshToken(token: string): void {
  localStorage.setItem(REFRESH_TOKEN_KEY, token);
}

export function clearTokens(): void {
  localStorage.removeItem(ACCESS_TOKEN_KEY);
  localStorage.removeItem(REFRESH_TOKEN_KEY);
}

const ADMIN_KEY = "ravenry.admin_api_key";

export function getAdminKey(): string | null {
  return sessionStorage.getItem(ADMIN_KEY);
}

export function setAdminKey(key: string): void {
  sessionStorage.setItem(ADMIN_KEY, key);
}

export function clearAdminKey(): void {
  sessionStorage.removeItem(ADMIN_KEY);
}
