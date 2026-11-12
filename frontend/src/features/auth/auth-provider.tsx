import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";

import { getCurrentUser } from "./api";
import {
  clearAdminKey,
  clearTokens,
  getAdminKey,
  getAccessToken,
  setAdminKey,
  setAccessToken,
  setAuthMode,
} from "./token-storage";
import { iamApiClient as apiClient } from "@/lib/api/clients";

import type { LoginRequest, LoginResponse, UserInfo } from "./types";
import { loginRequest } from "./api";

export type AuthMode = "user" | "admin" | null;

interface AuthContextValue {
  user: UserInfo | null;
  mode: AuthMode;
  isLoading: boolean;
  isAuthenticated: boolean;
  login: (input: LoginRequest) => Promise<void>;
  loginAdmin: (key: string) => Promise<void>;
  logout: () => void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<UserInfo | null>(null);
  const [mode, setMode] = useState<AuthMode>(null);
  const [isLoading, setIsLoading] = useState(true);

  const logout = useCallback(() => {
    clearTokens();
    clearAdminKey();
    setUser(null);
    setMode(null);
  }, []);

  const login = useCallback(async (input: LoginRequest) => {
    const response: LoginResponse = await loginRequest(input);
    setAccessToken(response.token);
    setAuthMode("user");

    try {
      const currentUser = await getCurrentUser();

      if (currentUser.is_admin) {
        throw new Error(
          "This account returned administrator identity. Use admin access instead.",
        );
      }

      clearAdminKey();
      setUser(currentUser);
      setMode("user");
    } catch (error) {
      clearTokens();
      setUser(null);
      setMode(null);
      throw error;
    }
  }, []);

  const loginAdmin = useCallback(async (key: string) => {
    clearTokens();
    setAdminKey(key);
    setAuthMode("admin");

    try {
      const response = await apiClient.get<UserInfo>("/auth/me");

      if (!response.data.is_admin) {
        throw new Error("This key does not have administrator access.");
      }

      setUser(response.data);
      setMode("admin");
    } catch (error) {
      clearAdminKey();
      setUser(null);
      setMode(null);
      throw error;
    }
  }, []);

  useEffect(() => {
    let cancelled = false;

    async function restoreSession() {
      try {
        const adminKey = getAdminKey();

        if (adminKey) {
          try {
            const response = await apiClient.get<UserInfo>("/auth/me");

            if (response.data.is_admin) {
              if (!cancelled) {
                setUser(response.data);
                setMode("admin");
              }
              return;
            }
          } catch {
            // Invalid or expired admin key: try a user session next.
          }

          clearAdminKey();
        }

        if (getAccessToken()) {
          const currentUser = await getCurrentUser();

          if (!currentUser.is_admin && !cancelled) {
            setUser(currentUser);
            setMode("user");
          } else if (currentUser.is_admin) {
            clearTokens();
          }
        }
      } catch {
        clearTokens();
        setUser(null);
        setMode(null);
      } finally {
        if (!cancelled) {
          setIsLoading(false);
        }
      }
    }

    void restoreSession();

    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <AuthContext.Provider
      value={{
        user,
        mode,
        isLoading,
        isAuthenticated: mode !== null && user !== null,
        login,
        loginAdmin,
        logout,
      }}
    >
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext);

  if (!context) {
    throw new Error("useAuth must be used within AuthProvider");
  }

  return context;
}
