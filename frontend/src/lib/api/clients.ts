import { createApiClient } from "./client";

function requiredEnv(name: string, value: string | undefined): string {
  if (!value) {
    throw new Error(`Missing ${name} in .env.local`);
  }

  return value;
}

export const iamApiClient = createApiClient(
  requiredEnv("VITE_IAM_API_BASE_URL", import.meta.env.VITE_IAM_API_BASE_URL),
);

export const huginnApiClient = createApiClient(
  requiredEnv(
    "VITE_HUGINN_API_BASE_URL",
    import.meta.env.VITE_HUGINN_API_BASE_URL,
  ),
);
