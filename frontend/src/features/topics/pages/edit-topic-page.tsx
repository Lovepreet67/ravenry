import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useParams } from "@tanstack/react-router";
import { ArrowLeft, Pencil } from "lucide-react";

import { getTopic, updateTopic } from "@/features/topics/api";
import { TopicForm } from "@/features/topics/components/topic-form";
import type { UpdateTopicFormValues } from "@/features/topics/schemas";
import { getApiErrorMessage } from "@/lib/api-error";

export function EditTopicPage() {
  const { tenantId, topicId } = useParams({
    from: "/tenants/$tenantId/topics/$topicId/edit",
  });

  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<string | null>(null);

  const topicQuery = useQuery({
    queryKey: ["topic", tenantId, topicId],
    queryFn: () => getTopic(tenantId, topicId),
  });

  const mutation = useMutation({
    mutationFn: (values: UpdateTopicFormValues) =>
      updateTopic(tenantId, topicId, {
        name: values.name.trim(),
        description: values.description?.trim(),
        is_active: values.is_active,
      }),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: ["topics", tenantId],
        }),
        queryClient.invalidateQueries({
          queryKey: ["topic", tenantId, topicId],
        }),
      ]);

      await navigate({
        to: "/tenants/$tenantId/topics/$topicId",
        params: { tenantId, topicId },
      });
    },
    onError: (error) => setServerError(getApiErrorMessage(error)),
  });

  async function handleSubmit(values: UpdateTopicFormValues) {
    setServerError(null);
    await mutation.mutateAsync(values);
  }

  if (topicQuery.isPending) {
    return (
      <p className="py-12 text-center text-sm text-muted-foreground">
        Loading topic...
      </p>
    );
  }

  if (topicQuery.isError) {
    return (
      <div className="space-y-4">
        <Link
          to="/tenants/$tenantId/topics"
          params={{ tenantId }}
          className="inline-flex items-center gap-2 text-sm text-muted-foreground"
        >
          <ArrowLeft size={15} />
          Back to topics
        </Link>
        <p role="alert" className="text-sm text-red-400">
          {getApiErrorMessage(topicQuery.error)}
        </p>
      </div>
    );
  }

  const topic = topicQuery.data;

  return (
    <div className="mx-auto max-w-3xl space-y-7">
      <Link
        to="/tenants/$tenantId/topics/$topicId"
        params={{ tenantId, topicId }}
        className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
      >
        <ArrowLeft size={15} />
        Back to topic
      </Link>

      <header className="flex items-start gap-4">
        <div className="rounded-xl border border-border bg-card p-3">
          <Pencil size={22} />
        </div>
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Edit topic</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Update {topic.name}
          </p>
        </div>
      </header>

      <section className="rounded-xl border border-border bg-card p-6">
        <TopicForm
          key={topic.id}
          mode="edit"
          initialValues={{
            name: topic.name,
            description: topic.description,
            is_active: topic.is_active,
          }}
          onSubmit={handleSubmit}
          isSubmitting={mutation.isPending}
          serverError={serverError}
        />
      </section>
    </div>
  );
}
