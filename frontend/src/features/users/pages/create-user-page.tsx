import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { ArrowLeft, UserPlus } from "lucide-react";

import { createUser } from "@/features/users/api";
import { UserForm } from "@/features/users/components/user-form";
import type { CreateUserFormValues } from "@/features/users/schemas";
import { getApiErrorMessage } from "@/lib/api-error";

export function CreateUserPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<string | null>(null);

  const mutation = useMutation({
    mutationFn: createUser,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["users"] });
      await navigate({ to: "/users" });
    },
    onError: (error) => {
      setServerError(getApiErrorMessage(error));
    },
  });

  async function handleSubmit(values: CreateUserFormValues) {
    setServerError(null);

    await mutation.mutateAsync({
      full_name: values.full_name.trim(),
      username: values.username.trim(),
      email: values.email.trim(),
      password: values.password,
    });
  }

  return (
    <div className="mx-auto max-w-3xl space-y-7">
      <Link
        to="/users"
        className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
      >
        <ArrowLeft size={15} />
        Back to users
      </Link>

      <header className="flex items-start gap-4">
        <div className="rounded-xl border border-border bg-card p-3">
          <UserPlus size={22} />
        </div>
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Create user</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Create a new account with its initial credentials.
          </p>
        </div>
      </header>

      <section className="rounded-xl border border-border bg-card p-6">
        <UserForm
          onSubmit={handleSubmit}
          isSubmitting={mutation.isPending}
          serverError={serverError}
        />
      </section>
    </div>
  );
}
