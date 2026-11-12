import axios, {
  type AxiosInstance,
  type InternalAxiosRequestConfig,
} from "axios";

import {
  clearTokens,
  getAccessToken,
  getRefreshToken,
  setAccessToken,
  setRefreshToken,
  getAdminKey,
  getAuthMode,
} from "@/features/auth/token-storage";

type RetryConfig = InternalAxiosRequestConfig & {
  _retry?: boolean;
};

interface RefreshResponse {
  token: string;
  refresh_token?: string;
}

let refreshPromise: Promise<string> | null = null;

function getRequiredEnv(name: string, value: string | undefined): string {
  if (!value) {
    throw new Error(`Missing ${name} in .env.local`);
  }

  return value;
}

const iamBaseURL = getRequiredEnv(
  "VITE_IAM_API_BASE_URL",
  import.meta.env.VITE_IAM_API_BASE_URL,
);

async function refreshAccessToken(): Promise<string> {
  const refreshToken = getRefreshToken();

  if (!refreshToken) {
    throw new Error("No refresh token available");
  }

  // Refresh must go through IAM, regardless of which service
  // originally returned the 401.
  const response = await axios.post<RefreshResponse>(
    `${iamBaseURL}/auth/refresh`,
    { refresh_token: refreshToken },
    { timeout: 15_000 },
  );

  const newAccessToken = response.data.token;

  setAccessToken(newAccessToken);

  if (response.data.refresh_token) {
    setRefreshToken(response.data.refresh_token);
  }

  return newAccessToken;
}

export function createApiClient(baseURL: string): AxiosInstance {
  const client = axios.create({
    baseURL,
    timeout: 15_000,
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
  });

  client.interceptors.request.use((config) => {
    const mode = getAuthMode();

    // Remove stale credentials from previous auth modes.
    config.headers.delete("Authorization");
    config.headers.delete("X-Admin-API-Key");

    if (mode === "admin") {
      const adminKey = getAdminKey();

      if (adminKey) {
        config.headers.set("X-Admin-API-Key", adminKey);
      }
    } else if (mode === "user") {
      const token = getAccessToken();

      if (token) {
        config.headers.set("Authorization", `Bearer ${token}`);
      }
    }

    return config;
  });

  client.interceptors.response.use(
    (response) => response,
    async (error: unknown) => {
      if (!axios.isAxiosError(error)) {
        return Promise.reject(error);
      }

      const config = error.config as RetryConfig | undefined;

      if (
        error.response?.status !== 401 ||
        !config ||
        config._retry ||
        config.url?.includes("/auth/login") ||
        config.url?.includes("/auth/refresh")
      ) {
        return Promise.reject(error);
      }

      // Admin API keys cannot be refreshed using user refresh tokens.
      if (getAuthMode() !== "user") {
        return Promise.reject(error);
      }

      config._retry = true;

      if (!getRefreshToken()) {
        clearTokens();
        return Promise.reject(error);
      }

      try {
        // Share one refresh operation across IAM and Huginn requests.
        refreshPromise ??= refreshAccessToken().finally(() => {
          refreshPromise = null;
        });

        const newToken = await refreshPromise;

        config.headers.set("Authorization", `Bearer ${newToken}`);

        return client(config);
      } catch (refreshError) {
        clearTokens();
        return Promise.reject(refreshError);
      }
    },
  );

  return client;
}
