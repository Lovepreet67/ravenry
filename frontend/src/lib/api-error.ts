
import axios from "axios";

interface ApiErrorBody {
  message?: string;
  error?: string;
}

export function getApiErrorMessage(error: unknown): string {
  if (axios.isAxiosError<ApiErrorBody>(error)) {
    if (error.response) {
      return (
        error.response.data?.message ??
        error.response.data?.error ??
        `Request failed with status ${error.response.status}`
      );
    }

    if (error.request) {
      return "Unable to reach the server. Check your connection and try again.";
    }
  }

  if (error instanceof Error) {
    return error.message;
  }

  return "An unexpected error occurred.";
}
