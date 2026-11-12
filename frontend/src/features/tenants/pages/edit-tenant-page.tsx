import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useParams } from "@tanstack/react-router";
import { ArrowLeft, Building2 } from "lucide-react";

import { getTenant, updateTenant } from "@/features/tenants/api";
import { TenantForm } from "@/features/tenants/components/tenant-form";
import type { UpdateTenantFormValues } from "@/features/tenants/schemas";
import { getApiErrorMessage } from "@/lib/api-error";

export function EditTenantPage() {
  const { tenantId } = useParams({ from: "/tenants/$tenantId/edit" });
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<string | null>(null);

  const tenantQuery = useQuery({
    queryKey: ["tenant", tenantId],
    queryFn: () => getTenant(tenantId),
  });

  const mutation = useMutation({
    mutationFn: (values: UpdateTenantFormValues) =>
      updateTenant(tenantId, values),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["tenants"] });
      await queryClient.invalidateQueries({
        queryKey: ["tenant", tenantId],
      });
      await navigate({ to: "/tenants" });
    },
    onError: (error) => {
      setServerError(getApiErrorMessage(error));
    },
  });

  async function handleSubmit(values: UpdateTenantFormValues) {
    setServerError(null);

    await mutation.mutateAsync({
      id: tenantId,
      name: values.name,
      is_active: values.is_active,
    });
  }

  if (tenantQuery.isPending) {
    return (
      <div className="py-12 text-sm text-muted-foreground">
        Loading tenant...
      </div>
    );
  }

  if (tenantQuery.isError) {
    return (
      <div className="space-y-4">
        <p className="text-sm text-red-400">
          {getApiErrorMessage(tenantQuery.error)}
        </p>
        <Link to="/tenants" className="text-sm text-primary hover:underline">
          Back to tenants
        </Link>
      </div>
    );
  }

  const tenant = tenantQuery.data;

  return (
    <div className="mx-auto max-w-3xl space-y-7">
      <Link
        to="/tenants"
        className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
      >
        <ArrowLeft size={15} />
        Back to tenants
      </Link>

      <div className="flex items-start gap-4">
        <div className="rounded-xl border border-border bg-card p-3">
          <Building2 size={22} />
        </div>
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Edit tenant</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Update tenant details and activation status.
          </p>
          <p className="mt-2 font-mono text-xs text-muted-foreground">
            {tenant.slug}
          </p>
        </div>
      </div>

      <div className="rounded-xl border border-border bg-card p-6">
        <TenantForm
          mode="edit"
          initialValues={tenant}
          onSubmit={handleSubmit}
          isSubmitting={mutation.isPending}
          serverError={serverError}
        />
      </div>
    </div>
  );
}
