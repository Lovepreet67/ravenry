import {
  Activity,
  ArrowUpRight,
  Building2,
  Clock3,
  ShieldCheck,
  Users,
} from "lucide-react";

const stats = [
  {
    label: "Total tenants",
    value: "—",
    description: "Organizations registered",
    icon: Building2,
  },
  {
    label: "Total users",
    value: "—",
    description: "Across all organizations",
    icon: Users,
  },
  {
    label: "Active memberships",
    value: "—",
    description: "User-tenant relationships",
    icon: ShieldCheck,
  },
  {
    label: "System status",
    value: "Local",
    description: "API connection not configured",
    icon: Activity,
  },
];

const quickLinks = [
  {
    title: "Manage tenants",
    description: "Create organizations and manage their settings.",
    icon: Building2,
  },
  {
    title: "Manage users",
    description: "View users and manage their access.",
    icon: Users,
  },
  {
    title: "Review memberships",
    description: "Inspect roles and organization membership.",
    icon: ShieldCheck,
  },
];

export function DashboardPage() {
  return (
    <div className="space-y-9">
      <section className="flex flex-col justify-between gap-4 sm:flex-row sm:items-end">
        <div className="space-y-2">
          <div className="flex items-center gap-2 text-xs font-medium uppercase tracking-[0.18em] text-primary">
            <span className="h-px w-5 bg-primary" />
            Control plane
          </div>
          <h1 className="text-3xl font-semibold tracking-tight sm:text-4xl">
            Overview
          </h1>
          <p className="max-w-xl text-sm leading-6 text-muted-foreground">
            Manage your organizations, users, and access policies from one
            workspace.
          </p>
        </div>

        <div className="flex w-fit items-center gap-2 rounded-md border border-border bg-card px-3 py-2 text-xs text-muted-foreground">
          <Clock3 size={14} />
          Development environment
        </div>
      </section>

      <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        {stats.map(({ label, value, description, icon: Icon }) => (
          <div
            key={label}
            className="group rounded-lg border border-border bg-card p-5 transition-colors hover:border-ring/40"
          >
            <div className="flex items-center justify-between">
              <p className="text-sm text-muted-foreground">{label}</p>
              <div className="flex size-9 items-center justify-center rounded-md border border-border bg-background text-primary">
                <Icon size={17} />
              </div>
            </div>
            <p className="mt-5 text-3xl font-semibold tracking-tight">
              {value}
            </p>
            <p className="mt-2 text-xs text-muted-foreground">{description}</p>
          </div>
        ))}
      </section>

      <section className="grid gap-6 xl:grid-cols-[1.4fr_1fr]">
        <div className="overflow-hidden rounded-lg border border-border bg-card">
          <div className="flex items-center justify-between border-b border-border px-5 py-4">
            <div>
              <h2 className="text-sm font-semibold">Workspace activity</h2>
              <p className="mt-1 text-xs text-muted-foreground">
                Activity will appear here once API integration is enabled.
              </p>
            </div>
            <Activity size={17} className="text-muted-foreground" />
          </div>

          <div className="flex min-h-64 flex-col items-center justify-center px-6 py-10 text-center">
            <div className="flex size-12 items-center justify-center rounded-full border border-border bg-background text-muted-foreground">
              <Activity size={20} />
            </div>
            <p className="mt-4 text-sm font-medium">No activity yet</p>
            <p className="mt-1 max-w-xs text-xs leading-5 text-muted-foreground">
              Connect your Axum API to display recent tenant, user, and
              membership events.
            </p>
          </div>
        </div>

        <div className="rounded-lg border border-border bg-card">
          <div className="border-b border-border px-5 py-4">
            <h2 className="text-sm font-semibold">Quick access</h2>
            <p className="mt-1 text-xs text-muted-foreground">
              Common administrative tasks
            </p>
          </div>

          <div className="divide-y divide-border">
            {quickLinks.map(({ title, description, icon: Icon }) => (
              <div
                key={title}
                className="group flex items-start gap-3 px-5 py-4 transition-colors hover:bg-accent/40"
              >
                <div className="flex size-9 shrink-0 items-center justify-center rounded-md border border-border bg-background text-primary">
                  <Icon size={17} />
                </div>
                <div className="min-w-0 flex-1">
                  <p className="text-sm font-medium">{title}</p>
                  <p className="mt-1 text-xs leading-5 text-muted-foreground">
                    {description}
                  </p>
                </div>
                <ArrowUpRight
                  size={15}
                  className="mt-1 text-muted-foreground transition-colors group-hover:text-primary"
                />
              </div>
            ))}
          </div>
        </div>
      </section>
    </div>
  );
}
