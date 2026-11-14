import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useParams } from "@tanstack/react-router";
import { ArrowLeft, Pencil } from "lucide-react";

import { getDestination, updateDestination } from "@/features/destinations/api";
import { DestinationForm } from "@/features/destinations/components/destination-form";
import type { UpdateDestinationFormValues } from "@/features/destinations/schemas";
import { getApiErrorMessage } from "@/lib/api-error";

export function EditDestinationPage() {
  const { tenantId, destinationId } = useParams({
    from: "/tenants/$tenantId/destinations/$destinationId/edit",
  });

  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<string | null>(null);

  const destinationQuery = useQuery({
    queryKey: ["destination", tenantId, destinationId],
    queryFn: () => getDestination(tenantId, destinationId),
  });

  const mutation = useMutation({
    mutationFn: (values: UpdateDestinationFormValues) =>
      updateDestination(tenantId, destinationId, {
        name: values.name.trim(),
        kind: values.kind,
        destination_url: values.destination_url.trim(),
        delivery_timeout_seconds: values.delivery_timeout_seconds,
        retry_delay_ms: values.retry_delay_ms,
        max_attempts: values.max_attempts,
        is_active: values.is_active,
      }),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: ["destinations", tenantId],
        }),
        queryClient.invalidateQueries({
          queryKey: ["destination", tenantId, destinationId],
        }),
      ]);

      await navigate({
        to: "/tenants/$tenantId/destinations/$destinationId",
        params: { tenantId, destinationId },
      });
    },
    onError: (error) => setServerError(getApiErrorMessage(error)),
  });

  async function handleSubmit(values: UpdateDestinationFormValues) {
    setServerError(null);
    await mutation.mutateAsync(values);
  }

  if (destinationQuery.isPending) {
    return (
      <p className="py-12 text-center text-sm text-muted-foreground">
        Loading destination...
      </p>
    );
  }

  if (destinationQuery.isError) {
    return (
      <div className="space-y-4">
        <Link
          to="/tenants/$tenantId/destinations"
          params={{ tenantId }}
          className="inline-flex items-center gap-2 text-sm text-muted-foreground"
        >
          <ArrowLeft size={15} />
          Back to destinations
        </Link>
        <p role="alert" className="text-sm text-red-400">
          {getApiErrorMessage(destinationQuery.error)}
        </p>
      </div>
    );
  }

  const destination = destinationQuery.data;

  return (
    <div className="mx-auto max-w-3xl space-y-7">
      <Link
        to="/tenants/$tenantId/destinations/$destinationId"
        params={{ tenantId, destinationId }}
        className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
      >
        <ArrowLeft size={15} />
        Back to destination
      </Link>

      <header className="flex items-start gap-4">
        <div className="rounded-xl border border-border bg-card p-3">
          <Pencil size={22} />
        </div>
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">
            Edit destination
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Update {destination.name} and its delivery settings.
          </p>
        </div>
      </header>

      <section className="rounded-xl border border-border bg-card p-6">
        <DestinationForm
          key={destination.id}
          mode="edit"
          initialValues={{
            name: destination.name,
            kind: destination.kind,
            destination_url: destination.destination_url,
            delivery_timeout_seconds: destination.delivery_timeout_seconds,
            retry_delay_ms: destination.retry_delay_ms,
            max_attempts: destination.max_attempts,
            is_active: destination.is_active,
          }}
          onSubmit={handleSubmit}
          isSubmitting={mutation.isPending}
          serverError={serverError}
        />
      </section>
    </div>
  );
}
