import {
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";

import { AppLayout } from "@/components/layout/app-layout";
// import { DashboardPage } from "@/features/dashboard/pages/dashboard-page";
import { TenantsPage } from "@/features/tenants/pages/tenants-page";
import { LoginPage } from "@/features/auth/pages/login-page";
import { AdminLoginPage } from "@/features/auth/pages/admin-login-page";
import { RequireAuth } from "@/features/auth/components/require-auth";
import { CreateTenantPage } from "@/features/tenants/pages/create-tenant-page";
import { EditTenantPage } from "@/features/tenants/pages/edit-tenant-page";
import { UsersPage } from "@/features/users/pages/users-page";
import { CreateUserPage } from "@/features/users/pages/create-user-page";
import { EditUserPage } from "@/features/users/pages/edit-user-page";
import { MembershipsPage } from "@/features/memberships/pages/memberships-page";
import { UserDetailsPage } from "@/features/users/pages/user-details-page";
import { DestinationsPage } from "@/features/destinations/pages/destinations-page";
import { CreateDestinationPage } from "@/features/destinations/pages/create-destination-page";
import { DestinationDetailsPage } from "@/features/destinations/pages/destination-details-page";
import { EditDestinationPage } from "@/features/destinations/pages/edit-destination-page";
import { TenantDetailsPage } from "@/features/tenants/pages/tenants-details-page";
import { TopicsPage } from "@/features/topics/pages/topics-page";
import { TopicDetailsPage } from "@/features/topics/pages/topic-details-page";
import { EditTopicPage } from "@/features/topics/pages/edit-topic-page";
import { CreateTopicPage } from "@/features/topics/pages/create-topic-page";
import { AdminDashboardPage } from "@/features/admin-dashboard/pages/dashboard-page";
import { EventsPage } from "@/features/events/pages/events-page";

function PlaceholderPage({ title }: { title: string }) {
  return (
    <div className="space-y-2">
      <p className="text-xs font-medium uppercase tracking-[0.2em] text-muted-foreground">
        Ravenry / Workspace
      </p>
      <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
      <p className="text-sm text-muted-foreground">
        This section will be implemented next.
      </p>
    </div>
  );
}

const rootRoute = createRootRoute({
  component: AppLayout,
});
const dashboardRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: () => (
    <RequireAuth requiredMode="any">
      <AdminDashboardPage />
    </RequireAuth>
  ),
});
const loginRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/login",
  component: LoginPage,
});

const adminLoginRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/admin-login",
  component: AdminLoginPage,
});

const tenantsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/tenants",
  component: () => (
    <RequireAuth requiredMode="admin">
      <TenantsPage />
    </RequireAuth>
  ),
});
const tenantsDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/tenants/$tenantId",
  component: () => (
    <RequireAuth requiredMode="any">
      <TenantDetailsPage />
    </RequireAuth>
  ),
});

const createTenantRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/tenants/create",
  component: () => (
    <RequireAuth requiredMode="admin">
      <CreateTenantPage />
    </RequireAuth>
  ),
});
const userDetailsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/users/$userId",
  component: () => (
    <RequireAuth requiredMode="admin">
      <UserDetailsPage />
    </RequireAuth>
  ),
});
const editTenantRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/tenants/$tenantId/edit",
  component: () => (
    <RequireAuth requiredMode="user" requiredRoles={["admin"]}>
      <EditTenantPage />
    </RequireAuth>
  ),
});

const usersRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/users",
  component: () => (
    <RequireAuth requiredMode="admin">
      <UsersPage />
    </RequireAuth>
  ),
});
const createUserRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/users/create",
  component: () => (
    <RequireAuth requiredMode="admin">
      <CreateUserPage />
    </RequireAuth>
  ),
});
const editUserRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/users/$userId/edit",
  component: () => (
    <RequireAuth requiredMode="user">
      <EditUserPage />
    </RequireAuth>
  ),
});
const membershipsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/memberships",
  component: () => (
    <RequireAuth requiredMode="admin">
      <MembershipsPage />
    </RequireAuth>
  ),
});

const destinationsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/tenants/$tenantId/destinations",
  component: () => (
    <RequireAuth requiredMode="user" requiredRoles={["admin"]}>
      <DestinationsPage />
    </RequireAuth>
  ),
});

const createDestinationRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/tenants/$tenantId/destinations/create",
  component: () => (
    <RequireAuth requiredMode="user" requiredRoles={["admin"]}>
      <CreateDestinationPage />
    </RequireAuth>
  ),
});

const destinationDetailsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/tenants/$tenantId/destinations/$destinationId",
  component: () => (
    <RequireAuth requiredMode="user" requiredRoles={["admin"]}>
      <DestinationDetailsPage />
    </RequireAuth>
  ),
});

const editDestinationRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/tenants/$tenantId/destinations/$destinationId/edit",
  component: () => (
    <RequireAuth requiredMode="user" requiredRoles={["admin"]}>
      <EditDestinationPage />
    </RequireAuth>
  ),
});

// Topics
const topicsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/tenants/$tenantId/topics",
  component: () => (
    <RequireAuth requiredMode="user" requiredRoles={["admin"]}>
      <TopicsPage />
    </RequireAuth>
  ),
});

const createTopicRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/tenants/$tenantId/topics/create",
  component: () => (
    <RequireAuth requiredMode="user" requiredRoles={["admin"]}>
      <CreateTopicPage />
    </RequireAuth>
  ),
});

const topicDetailsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/tenants/$tenantId/topics/$topicId",
  component: () => (
    <RequireAuth requiredMode="user" requiredRoles={["admin"]}>
      <TopicDetailsPage />
    </RequireAuth>
  ),
});

const editTopicRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/tenants/$tenantId/topics/$topicId/edit",
  component: () => (
    <RequireAuth requiredMode="user" requiredRoles={["admin"]}>
      <EditTopicPage />
    </RequireAuth>
  ),
});

// Topics end

// events

const tenantEventsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/tenants/$tenantId/events",
  component: () => {
    const { tenantId } = tenantEventsRoute.useParams();

    return (
      <RequireAuth requiredMode="any">
        <EventsPage tenantId={tenantId} />
      </RequireAuth>
    );
  },
});

const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  component: () => <PlaceholderPage title="Settings" />,
});

const routeTree = rootRoute.addChildren([
  loginRoute,
  adminLoginRoute,
  dashboardRoute,
  tenantsRoute,
  tenantsDetailRoute,
  createTenantRoute,
  editTenantRoute,
  usersRoute,
  createUserRoute,
  userDetailsRoute,
  editUserRoute,
  membershipsRoute,
  destinationsRoute,
  createDestinationRoute,
  destinationDetailsRoute,
  editDestinationRoute,
  topicsRoute,
  createTopicRoute,
  topicDetailsRoute,
  editTopicRoute,
  tenantEventsRoute,
  settingsRoute,
]);

export const router = createRouter({
  routeTree,
  defaultPreload: "intent",
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
