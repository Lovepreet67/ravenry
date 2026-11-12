import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useParams } from "@tanstack/react-router";
import { ArrowLeft, Plus } from "lucide-react";

import { createTopic } from "@/features/topics/api";
import { TopicForm } from "@/features/topics/components/topic-form";
import type { CreateTopicFormValues } from "@/features/topics/schemas";
import { getApiErrorMessage } from "@/lib/api-error";

export function CreateTopicPage() {
  const { tenantId } = useParams({
    from: "/tenants/$tenantId/topics/create",
  });
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<string | null>(null);

  const mutation = useMutation({
    mutationFn: (values: CreateTopicFormValues) =>
      createTopic(tenantId, {
        ...values,
        name: values.name.trim(),
        path: values.path.trim(),
        ...(values.description?.trim()
          ? { description: values.description.trim() }
          : {}),
      }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({
        queryKey: ["topics", tenantId],
      });
      await navigate({
        to: "/tenants/$tenantId/topics",
        params: { tenantId },
      });
    },
    onError: (error) => setServerError(getApiErrorMessage(error)),
  });

  async function handleSubmit(values: CreateTopicFormValues) {
    setServerError(null);
    await mutation.mutateAsync(values);
  }

  return (
    <div className="mx-auto max-w-3xl space-y-7">
      <Link
        to="/tenants/$tenantId/topics"
        params={{ tenantId }}
        className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
      >
        <ArrowLeft size={15} />
        Back to topics
      </Link>

      <header className="flex items-start gap-4">
        <div className="rounded-xl border border-border bg-card p-3">
          <Plus size={22} />
        </div>
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">
            Create topic
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Configure an endpoint to receive events for this tenant.
          </p>
        </div>
      </header>

      <section className="rounded-xl border border-border bg-card p-6">
        <TopicForm
          mode="create"
          onSubmit={handleSubmit}
          isSubmitting={mutation.isPending}
          serverError={serverError}
        />
      </section>
    </div>
  );
}
