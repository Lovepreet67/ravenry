import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";

import { createTenant } from "@/features/tenants/api";
import { TenantForm } from "@/features/tenants/components/tenant-form";
import type { CreateTenantFormValues } from "@/features/tenants/schemas";
import { getApiErrorMessage } from "@/lib/api-error";

export function CreateTenantPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<string | null>(null);

  const mutation = useMutation({
    mutationFn: createTenant,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["tenants"] });
      await navigate({ to: "/tenants" });
    },
    onError: (error) => {
      setServerError(getApiErrorMessage(error));
    },
  });

  async function handleSubmit(values: CreateTenantFormValues) {
    setServerError(null);

    const defaultRoleSlug = values.default_role_slug?.trim();

    await mutation.mutateAsync({
      name: values.name,
      slug: values.slug,
      ...(defaultRoleSlug ? { default_role_slug: defaultRoleSlug } : {}),
    });
  }

  return (
    <TenantForm
      mode="create"
      onSubmit={handleSubmit}
      isSubmitting={mutation.isPending}
      serverError={serverError}
    />
  );
}
