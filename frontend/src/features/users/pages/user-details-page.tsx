import { useQuery } from "@tanstack/react-query";
import { Link, useParams } from "@tanstack/react-router";
import {
  ArrowLeft,
  Building2,
  CalendarDays,
  Mail,
  Pencil,
  UserRound,
} from "lucide-react";

import { getUser } from "@/features/users/api";
import { getUserMemberships } from "@/features/memberships/api";
import { getApiErrorMessage } from "@/lib/api-error";

function formatDate(value: string) {
  const date = new Date(value);

  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

export function UserDetailsPage() {
  const { userId } = useParams({
    from: "/users/$userId",
  });

  const userQuery = useQuery({
    queryKey: ["user", userId],
    queryFn: () => getUser(userId),
  });

  const membershipsQuery = useQuery({
    queryKey: ["user-memberships", userId],
    queryFn: () => getUserMemberships(userId),
  });

  if (userQuery.isPending) {
    return (
      <p className="py-12 text-center text-sm text-muted-foreground">
        Loading user...
      </p>
    );
  }

  if (userQuery.isError) {
    return (
      <div className="space-y-4">
        <Link
          to="/users"
          className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
        >
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
  const memberships = membershipsQuery.data?.memberships ?? [];

  return (
    <div className="mx-auto max-w-6xl space-y-7">
      <Link
        to="/users"
        className="inline-flex items-center gap-2 text-sm text-muted-foreground transition-colors hover:text-foreground"
      >
        <ArrowLeft size={15} />
        Back to users
      </Link>

      <header className="flex flex-wrap items-start justify-between gap-4">
        <div className="flex items-start gap-4">
          <div className="rounded-xl border border-border bg-card p-3">
            <UserRound size={24} />
          </div>
          <div>
            <h1 className="text-2xl font-semibold tracking-tight">
              {user.full_name}
            </h1>
            <p className="mt-1 text-sm text-muted-foreground">
              @{user.username}
            </p>
          </div>
        </div>

        <Link
          to="/users/$userId/edit"
          params={{ userId: user.id }}
          className="inline-flex items-center gap-2 rounded-lg border border-border px-4 py-2.5 text-sm font-medium transition-colors hover:bg-card"
        >
          <Pencil size={15} />
          Edit user
        </Link>
      </header>

      <section className="space-y-5 rounded-xl border border-border bg-card p-6">
        <h2 className="text-lg font-semibold">Account information</h2>

        <div className="grid gap-6 sm:grid-cols-2">
          <div className="flex items-start gap-3">
            <Mail size={18} className="mt-0.5 text-muted-foreground" />
            <div className="min-w-0">
              <p className="text-xs text-muted-foreground">Email</p>
              <p className="mt-1 break-all text-sm font-medium">{user.email}</p>
            </div>
          </div>

          <div className="flex items-start gap-3">
            <UserRound size={18} className="mt-0.5 text-muted-foreground" />
            <div>
              <p className="text-xs text-muted-foreground">Username</p>
              <p className="mt-1 text-sm font-medium">{user.username}</p>
            </div>
          </div>

          <div className="flex items-start gap-3">
            <CalendarDays size={18} className="mt-0.5 text-muted-foreground" />
            <div>
              <p className="text-xs text-muted-foreground">Created</p>
              <p className="mt-1 text-sm font-medium">
                {formatDate(user.created_at)}
              </p>
            </div>
          </div>

          <div className="flex items-start gap-3">
            <CalendarDays size={18} className="mt-0.5 text-muted-foreground" />
            <div>
              <p className="text-xs text-muted-foreground">Last updated</p>
              <p className="mt-1 text-sm font-medium">
                {formatDate(user.updated_at)}
              </p>
            </div>
          </div>
        </div>
      </section>

      <section className="space-y-4">
        <div className="flex items-center justify-between gap-3">
          <div>
            <h2 className="text-lg font-semibold">Tenant memberships</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Tenants this user can access and their assigned roles.
            </p>
          </div>

          <span className="rounded-full border border-border px-3 py-1 text-sm text-muted-foreground">
            {membershipsQuery.isSuccess ? memberships.length : "—"}
          </span>
        </div>

        {membershipsQuery.isPending ? (
          <div className="rounded-xl border border-border p-8 text-center text-sm text-muted-foreground">
            Loading memberships...
          </div>
        ) : membershipsQuery.isError ? (
          <div className="rounded-xl border border-border p-6">
            <p role="alert" className="text-sm text-red-400">
              Could not load memberships:{" "}
              {getApiErrorMessage(membershipsQuery.error)}
            </p>
          </div>
        ) : memberships.length === 0 ? (
          <div className="rounded-xl border border-border p-8 text-center">
            <Building2 size={24} className="mx-auto text-muted-foreground" />
            <p className="mt-3 text-sm font-medium">No tenant memberships</p>
            <p className="mt-1 text-sm text-muted-foreground">
              This user has not been assigned to any tenant yet.
            </p>
            <Link
              to="/memberships"
              className="mt-4 inline-flex text-sm font-medium text-primary hover:underline"
            >
              Manage memberships
            </Link>
          </div>
        ) : (
          <div className="overflow-x-auto rounded-xl border border-border">
            <table className="w-full text-left text-sm">
              <thead className="bg-card text-muted-foreground">
                <tr>
                  <th className="px-5 py-3 font-medium">Tenant</th>
                  <th className="px-5 py-3 font-medium">Tenant slug</th>
                  <th className="px-5 py-3 font-medium">Role</th>
                  <th className="px-5 py-3 font-medium">Role slug</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border">
                {memberships.map((membership) => (
                  <tr key={membership.tenant_id}>
                    <td className="px-5 py-4 font-medium">
                      <div className="flex items-center gap-2">
                        <Building2
                          size={16}
                          className="text-muted-foreground"
                        />
                        {membership.tenant_name}
                      </div>
                    </td>
                    <td className="px-5 py-4 text-muted-foreground">
                      {membership.tenant_slug}
                    </td>
                    <td className="px-5 py-4">
                      <span className="rounded-md border border-border px-2 py-1 text-xs">
                        {membership.role_title}
                      </span>
                    </td>
                    <td className="px-5 py-4 text-muted-foreground">
                      {membership.role_slug}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </div>
  );
}
