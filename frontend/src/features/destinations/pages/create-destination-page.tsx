import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useParams } from "@tanstack/react-router";
import { ArrowLeft, Plus } from "lucide-react";

import { createDestination } from "@/features/destinations/api";
import { DestinationForm } from "@/features/destinations/components/destination-form";
import type { CreateDestinationFormValues } from "@/features/destinations/schemas";
import { getApiErrorMessage } from "@/lib/api-error";

export function CreateDestinationPage() {
  const { tenantId } = useParams({
    from: "/tenants/$tenantId/destinations/create",
  });
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<string | null>(null);

  const mutation = useMutation({
    mutationFn: (values: CreateDestinationFormValues) =>
      createDestination(tenantId, {
        ...values,
        name: values.name.trim(),
        destination_url: values.destination_url.trim(),
        ...(values.description?.trim()
          ? { description: values.description.trim() }
          : {}),
      }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({
        queryKey: ["destinations", tenantId],
      });
      await navigate({
        to: "/tenants/$tenantId/destinations",
        params: { tenantId },
      });
    },
    onError: (error) => setServerError(getApiErrorMessage(error)),
  });

  async function handleSubmit(values: CreateDestinationFormValues) {
    setServerError(null);
    await mutation.mutateAsync(values);
  }

  return (
    <div className="mx-auto max-w-3xl space-y-7">
      <Link
        to="/tenants/$tenantId/destinations"
        params={{ tenantId }}
        className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
      >
        <ArrowLeft size={15} />
        Back to destinations
      </Link>

      <header className="flex items-start gap-4">
        <div className="rounded-xl border border-border bg-card p-3">
          <Plus size={22} />
        </div>
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">
            Create destination
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Configure an endpoint to receive events for this tenant.
          </p>
        </div>
      </header>

      <section className="rounded-xl border border-border bg-card p-6">
        <DestinationForm
          mode="create"
          onSubmit={handleSubmit}
          isSubmitting={mutation.isPending}
          serverError={serverError}
        />
      </section>
    </div>
  );
}
