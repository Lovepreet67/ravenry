import { useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { Link, useNavigate } from "@tanstack/react-router";
import { z } from "zod";
import { KeyRound, LoaderCircle } from "lucide-react";

import { getApiErrorMessage } from "@/lib/api-error";
import { useAuth } from "@/features/auth/auth-provider";

const adminLoginSchema = z.object({
  adminKey: z.string().trim().min(1, "Admin API key is required"),
});

type AdminLoginFormValues = z.infer<typeof adminLoginSchema>;

export function AdminLoginForm() {
  const navigate = useNavigate();
  const [serverError, setServerError] = useState<string | null>(null);
  const { loginAdmin } = useAuth();

  async function onSubmit(values: AdminLoginFormValues) {
    setServerError(null);

    try {
      await loginAdmin(values.adminKey);
      await navigate({ to: "/tenants" });
    } catch (error) {
      setServerError(getApiErrorMessage(error));
    }
  }

  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<AdminLoginFormValues>({
    resolver: zodResolver(adminLoginSchema),
    defaultValues: { adminKey: "" },
  });

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="space-y-5">
      <div className="space-y-1.5">
        <label htmlFor="adminKey" className="text-sm font-medium">
          Admin API key
        </label>
        <input
          id="adminKey"
          type="password"
          autoComplete="current-password"
          placeholder="Enter your admin key"
          {...register("adminKey")}
          aria-invalid={!!errors.adminKey}
          className="h-11 w-full rounded-lg border border-input bg-background px-3 text-sm outline-none transition placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-ring"
        />
        {errors.adminKey && (
          <p className="text-xs text-destructive">{errors.adminKey.message}</p>
        )}
        <p className="text-xs text-muted-foreground">
          The key is kept in this browser tab's session storage.
        </p>
      </div>

      {serverError && (
        <p
          role="alert"
          className="rounded-lg border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
        >
          {serverError}
        </p>
      )}

      <button
        type="submit"
        disabled={isSubmitting}
        className="flex h-11 w-full items-center justify-center gap-2 rounded-lg bg-primary text-sm font-semibold text-primary-foreground transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
      >
        {isSubmitting ? (
          <LoaderCircle size={16} className="animate-spin" />
        ) : (
          <KeyRound size={16} />
        )}
        {isSubmitting ? "Verifying key..." : "Continue as administrator"}
      </button>

      <p className="text-center text-sm text-muted-foreground">
        Signing in as a user?{" "}
        <Link to="/login" className="font-medium text-primary hover:underline">
          User login
        </Link>
      </p>
    </form>
  );
}
