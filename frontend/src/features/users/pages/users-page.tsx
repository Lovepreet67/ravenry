import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { Plus, RefreshCw, Search, Users } from "lucide-react";

import { listUsers } from "@/features/users/api";
import type { UserListParams } from "@/features/users/types";
import { getApiErrorMessage } from "@/lib/api-error";

const PAGE_SIZE = 10;

export function UsersPage() {
  const [pageNo, setPageNo] = useState(1);
  const [fullName, setFullName] = useState("");
  const [username, setUsername] = useState("");
  const [email, setEmail] = useState("");
  const [appliedFilters, setAppliedFilters] = useState({
    full_name: "",
    username: "",
    email: "",
  });

  const params: UserListParams = {
    page_no: pageNo,
    page_size: PAGE_SIZE,
    ...(appliedFilters.full_name
      ? { full_name: appliedFilters.full_name }
      : {}),
    ...(appliedFilters.username ? { username: appliedFilters.username } : {}),
    ...(appliedFilters.email ? { email: appliedFilters.email } : {}),
  };

  const usersQuery = useQuery({
    queryKey: ["users", params],
    queryFn: () => listUsers(params),
  });

  function applyFilters(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setPageNo(1);
    setAppliedFilters({
      full_name: fullName.trim(),
      username: username.trim(),
      email: email.trim(),
    });
  }

  const result = usersQuery.data;
  const pageInfo = result?.page_info;
  const totalPages = pageInfo
    ? Math.max(1, Math.ceil(pageInfo.total_rows / pageInfo.page_size))
    : 1;

  return (
    <div className="mx-auto max-w-7xl space-y-7">
      <header className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-3">
            <div className="rounded-xl border border-border bg-card p-3">
              <Users size={22} />
            </div>
            <div>
              <h1 className="text-2xl font-semibold tracking-tight">Users</h1>
              <p className="mt-1 text-sm text-muted-foreground">
                Manage accounts and user details.
              </p>
            </div>
          </div>
        </div>

        <Link
          to="/users/create"
          className="inline-flex items-center gap-2 rounded-lg bg-primary px-4 py-2.5 text-sm font-semibold text-primary-foreground hover:opacity-90"
        >
          <Plus size={16} />
          Create user
        </Link>
      </header>

      <form
        onSubmit={applyFilters}
        className="grid gap-3 rounded-xl border border-border bg-card p-4 md:grid-cols-4"
      >
        <input
          value={fullName}
          onChange={(event) => setFullName(event.target.value)}
          placeholder="Filter by full name"
          className="rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none focus:border-primary"
        />
        <input
          value={username}
          onChange={(event) => setUsername(event.target.value)}
          placeholder="Filter by username"
          className="rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none focus:border-primary"
        />
        <input
          value={email}
          onChange={(event) => setEmail(event.target.value)}
          placeholder="Filter by email"
          className="rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none focus:border-primary"
        />
        <button
          type="submit"
          className="inline-flex items-center justify-center gap-2 rounded-lg border border-border px-4 py-2 text-sm hover:bg-muted"
        >
          <Search size={15} />
          Apply filters
        </button>
      </form>

      <section className="overflow-hidden rounded-xl border border-border bg-card">
        <div className="flex items-center justify-between border-b border-border px-5 py-4">
          <div>
            <h2 className="text-sm font-semibold">All users</h2>
            <p className="mt-1 text-xs text-muted-foreground">
              {pageInfo
                ? `${pageInfo.total_rows} total users`
                : "Loading users..."}
            </p>
          </div>

          <button
            type="button"
            onClick={() => void usersQuery.refetch()}
            disabled={usersQuery.isFetching}
            aria-label="Refresh users"
            className="rounded-lg border border-border p-2 hover:bg-muted disabled:opacity-50"
          >
            <RefreshCw
              size={15}
              className={usersQuery.isFetching ? "animate-spin" : ""}
            />
          </button>
        </div>

        {usersQuery.isPending ? (
          <div className="p-8 text-center text-sm text-muted-foreground">
            Loading users...
          </div>
        ) : usersQuery.isError ? (
          <div className="p-8 text-center text-sm text-red-400">
            {getApiErrorMessage(usersQuery.error)}
          </div>
        ) : result?.list.length === 0 ? (
          <div className="p-8 text-center text-sm text-muted-foreground">
            No users found. Try changing your filters.
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-left text-sm">
              <thead className="border-b border-border bg-background/50 text-xs text-muted-foreground">
                <tr>
                  <th className="px-5 py-3 font-medium">Name</th>
                  <th className="px-5 py-3 font-medium">Username</th>
                  <th className="px-5 py-3 font-medium">Email</th>
                  <th className="px-5 py-3 font-medium">Created</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border">
                {result?.list.map((user) => (
                  <tr key={user.id} className="hover:bg-muted/30">
                    <td className="px-5 py-4">
                      <Link
                        to="/users/$userId"
                        params={{ userId: user.id }}
                        className="font-medium hover:text-primary"
                      >
                        {user.full_name}
                      </Link>
                    </td>
                    <td className="px-5 py-4 text-muted-foreground">
                      {user.username}
                    </td>
                    <td className="px-5 py-4 text-muted-foreground">
                      {user.email}
                    </td>
                    <td className="px-5 py-4 text-muted-foreground">
                      {new Date(user.created_at).toLocaleDateString()}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        <div className="flex flex-wrap items-center justify-between gap-3 border-t border-border px-5 py-4">
          <span className="text-xs text-muted-foreground">
            Page {pageInfo?.page_no ?? pageNo} of {totalPages}
          </span>

          <div className="flex gap-2">
            <button
              type="button"
              disabled={pageNo <= 1 || usersQuery.isFetching}
              onClick={() => setPageNo((page) => page - 1)}
              className="rounded-lg border border-border px-3 py-2 text-xs disabled:opacity-40"
            >
              Previous
            </button>
            <button
              type="button"
              disabled={pageNo >= totalPages || usersQuery.isFetching}
              onClick={() => setPageNo((page) => page + 1)}
              className="rounded-lg border border-border px-3 py-2 text-xs disabled:opacity-40"
            >
              Next
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}
