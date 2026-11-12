import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useParams } from "@tanstack/react-router";
import { ArrowLeft, UserRound } from "lucide-react";

import { getUser, updateUser } from "@/features/users/api";
import { EditUserForm } from "@/features/users/components/edit-user-form";
import type { UpdateUserFormValues } from "@/features/users/schemas";
import { getApiErrorMessage } from "@/lib/api-error";

export function EditUserPage() {
  const { userId } = useParams({
    from: "/users/$userId/edit",
  });

  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<string | null>(null);

  const userQuery = useQuery({
    queryKey: ["user", userId],
    queryFn: () => getUser(userId),
  });

  const mutation = useMutation({
    mutationFn: (values: UpdateUserFormValues) =>
      updateUser(userId, {
        id: userId,
        full_name: values.full_name.trim(),
      }),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["users"] }),
        queryClient.invalidateQueries({ queryKey: ["user", userId] }),
      ]);

      await navigate({ to: "/users" });
    },
    onError: (error) => {
      setServerError(getApiErrorMessage(error));
    },
  });

  async function handleSubmit(values: UpdateUserFormValues) {
    setServerError(null);
    await mutation.mutateAsync(values);
  }

  if (userQuery.isPending) {
    return (
      <div className="py-12 text-center text-sm text-muted-foreground">
        Loading user...
      </div>
    );
  }

  if (userQuery.isError) {
    return (
      <div className="space-y-4">
        <Link to="/users" className="inline-flex items-center gap-2 text-sm">
          <ArrowLeft size={15} />
          Back to users
        </Link>
        <p role="alert" className="text-sm text-red-400">
          {getApiErrorMessage(userQuery.error)}
        </p>
      </div>
    );
  }

  const user = userQuery.data;

  return (
    <div className="mx-auto max-w-3xl space-y-7">
      <Link
        to="/users"
        className="inline-flex items-center gap-2 text-sm text-muted-foreground transition-colors hover:text-foreground"
      >
        <ArrowLeft size={15} />
        Back to users
      </Link>

      <header className="flex items-start gap-4">
        <div className="rounded-xl border border-border bg-card p-3">
          <UserRound size={22} />
        </div>
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Edit user</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Update the account details for {user.username}.
          </p>
        </div>
      </header>

      <section className="space-y-5 rounded-xl border border-border bg-card p-6">
        <div className="grid gap-4 sm:grid-cols-2">
          <div>
            <p className="text-xs text-muted-foreground">Username</p>
            <p className="mt-1 text-sm font-medium">{user.username}</p>
          </div>
          <div>
            <p className="text-xs text-muted-foreground">Email</p>
            <p className="mt-1 break-all text-sm font-medium">{user.email}</p>
          </div>
        </div>

        <div className="border-t border-border pt-5">
          <EditUserForm
            initialValues={{ full_name: user.full_name }}
            onSubmit={handleSubmit}
            isSubmitting={mutation.isPending}
            serverError={serverError}
          />
        </div>
      </section>
    </div>
  );
}
