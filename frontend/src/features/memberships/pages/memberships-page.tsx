import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  getUserMemberships,
  upsertMembership,
} from "@/features/memberships/api";
import { listUsers } from "@/features/users/api";
import { listTenants } from "@/features/tenants/api";
import { listRoles } from "@/features/roles/api";
import { getApiErrorMessage } from "@/lib/api-error";

const inputClass =
  "w-full rounded-lg border border-border bg-background px-3 py-2.5 text-sm outline-none focus:border-primary";

export function MembershipsPage() {
  const queryClient = useQueryClient();
  const [selectedUserId, setSelectedUserId] = useState("");
  const [tenantId, setTenantId] = useState("");
  const [roleId, setRoleId] = useState("");
  const [serverError, setServerError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  const usersQuery = useQuery({
    queryKey: ["users", "membership-options"],
    queryFn: () => listUsers({ page_no: 1, page_size: 100 }),
  });

  const tenantsQuery = useQuery({
    queryKey: ["tenants", "membership-options"],
    queryFn: () => listTenants({ page_no: 1, page_size: 100 }),
  });

  const rolesQuery = useQuery({
    queryKey: ["roles"],
    queryFn: listRoles,
  });

  const membershipsQuery = useQuery({
    queryKey: ["user-memberships", selectedUserId],
    queryFn: () => getUserMemberships(selectedUserId),
    enabled: Boolean(selectedUserId),
  });

  const mutation = useMutation({
    mutationFn: () =>
      upsertMembership(tenantId, {
        user_id: selectedUserId,
        role_id: roleId,
      }),
    onSuccess: async () => {
      setServerError(null);
      setSuccessMessage("Membership saved successfully.");

      await queryClient.invalidateQueries({
        queryKey: ["user-memberships", selectedUserId],
      });
    },
    onError: (error) => {
      setSuccessMessage(null);
      setServerError(getApiErrorMessage(error));
    },
  });

  const users = usersQuery.data?.list ?? [];
  const tenants = tenantsQuery.data?.list ?? [];
  const roles = rolesQuery.data ?? [];
  const memberships = membershipsQuery.data?.memberships ?? [];

  const isLoadingOptions =
    usersQuery.isPending || tenantsQuery.isPending || rolesQuery.isPending;

  const optionsError =
    usersQuery.error ?? tenantsQuery.error ?? rolesQuery.error;

  function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setServerError(null);
    setSuccessMessage(null);

    if (!selectedUserId || !tenantId || !roleId) {
      setServerError("Select a user, tenant, and role.");
      return;
    }

    mutation.mutate();
  }

  return (
    <div className="mx-auto max-w-6xl space-y-7">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight">Memberships</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Manage user access and roles across tenants.
        </p>
      </header>

      {isLoadingOptions && (
        <p className="text-sm text-muted-foreground">
          Loading users, tenants, and roles...
        </p>
      )}

      {optionsError && (
        <p role="alert" className="text-sm text-red-400">
          {getApiErrorMessage(optionsError)}
        </p>
      )}

      {!isLoadingOptions && !optionsError && (
        <>
          <section className="space-y-5 rounded-xl border border-border bg-card p-6">
            <h2 className="text-lg font-semibold">Assign membership</h2>

            <form onSubmit={handleSubmit} className="space-y-5">
              <div className="grid gap-5 md:grid-cols-3">
                <div className="space-y-2">
                  <label htmlFor="user_id" className="text-sm font-medium">
                    User
                  </label>
                  <select
                    id="user_id"
                    value={selectedUserId}
                    onChange={(event) => {
                      setSelectedUserId(event.target.value);
                      setSuccessMessage(null);
                      setServerError(null);
                    }}
                    className={inputClass}
                    required
                  >
                    <option value="">Select a user</option>
                    {users.map((user) => (
                      <option key={user.id} value={user.id}>
                        {user.full_name} ({user.username})
                      </option>
                    ))}
                  </select>
                  {users.length === 0 && (
                    <p className="text-xs text-muted-foreground">
                      No users found on this page.
                    </p>
                  )}
                </div>

                <div className="space-y-2">
                  <label htmlFor="tenant_id" className="text-sm font-medium">
                    Tenant
                  </label>
                  <select
                    id="tenant_id"
                    value={tenantId}
                    onChange={(event) => {
                      setTenantId(event.target.value);
                      setSuccessMessage(null);
                      setServerError(null);
                    }}
                    className={inputClass}
                    required
                  >
                    <option value="">Select a tenant</option>
                    {tenants.map((tenant) => (
                      <option key={tenant.id} value={tenant.id}>
                        {tenant.name} ({tenant.slug})
                      </option>
                    ))}
                  </select>
                </div>

                <div className="space-y-2">
                  <label htmlFor="role_id" className="text-sm font-medium">
                    Role
                  </label>
                  <select
                    id="role_id"
                    value={roleId}
                    onChange={(event) => {
                      setRoleId(event.target.value);
                      setSuccessMessage(null);
                      setServerError(null);
                    }}
                    className={inputClass}
                    required
                  >
                    <option value="">Select a role</option>
                    {roles.map((role) => (
                      <option key={role.id} value={role.id}>
                        {role.title} ({role.slug})
                      </option>
                    ))}
                  </select>
                </div>
              </div>

              {serverError && (
                <p role="alert" className="text-sm text-red-400">
                  {serverError}
                </p>
              )}

              {successMessage && (
                <p role="status" className="text-sm text-emerald-400">
                  {successMessage}
                </p>
              )}

              <div className="flex justify-end">
                <button
                  type="submit"
                  disabled={
                    mutation.isPending ||
                    !selectedUserId ||
                    !tenantId ||
                    !roleId
                  }
                  className="rounded-lg bg-primary px-4 py-2.5 text-sm font-semibold text-background transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
                >
                  {mutation.isPending ? "Saving..." : "Save membership"}
                </button>
              </div>
            </form>
          </section>

          <section className="space-y-4">
            <div>
              <h2 className="text-lg font-semibold">User memberships</h2>
              <p className="mt-1 text-sm text-muted-foreground">
                Select a user above to inspect their tenant access.
              </p>
            </div>

            {!selectedUserId ? (
              <div className="rounded-xl border border-border p-8 text-center text-sm text-muted-foreground">
                Select a user to view their memberships.
              </div>
            ) : membershipsQuery.isPending ? (
              <p className="py-8 text-center text-sm text-muted-foreground">
                Loading memberships...
              </p>
            ) : membershipsQuery.isError ? (
              <p role="alert" className="text-sm text-red-400">
                {getApiErrorMessage(membershipsQuery.error)}
              </p>
            ) : memberships.length === 0 ? (
              <div className="rounded-xl border border-border p-8 text-center">
                <p className="text-sm text-muted-foreground">
                  This user has no memberships yet.
                </p>
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
                          {membership.tenant_name}
                        </td>
                        <td className="px-5 py-4 text-muted-foreground">
                          {membership.tenant_slug}
                        </td>
                        <td className="px-5 py-4">{membership.role_title}</td>
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
        </>
      )}
    </div>
  );
}
