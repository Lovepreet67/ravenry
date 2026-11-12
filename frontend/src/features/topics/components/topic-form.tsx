import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";

import {
  createTopicSchema,
  updateTopicSchema,
  type CreateTopicFormValues,
  type UpdateTopicFormValues,
} from "@/features/topics/schemas";

interface CommonProps {
  isSubmitting: boolean;
  serverError: string | null;
}

interface CreateProps extends CommonProps {
  mode: "create";
  onSubmit: (values: CreateTopicFormValues) => Promise<void>;
}

interface EditProps extends CommonProps {
  mode: "edit";
  initialValues: UpdateTopicFormValues;
  onSubmit: (values: UpdateTopicFormValues) => Promise<void>;
}

type Props = CreateProps | EditProps;

const inputClass =
  "w-full rounded-lg border border-border bg-background px-3 py-2.5 text-sm outline-none focus:border-primary";

function FieldError({ message }: { message?: string }) {
  if (!message) return null;

  return <p className="text-sm text-red-400">{message}</p>;
}

export function TopicForm(props: Props) {
  if (props.mode === "create") {
    return <CreateForm {...props} />;
  }

  return <EditForm {...props} />;
}

function CreateForm({ onSubmit, isSubmitting, serverError }: CreateProps) {
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<CreateTopicFormValues>({
    resolver: zodResolver(createTopicSchema),
    defaultValues: {
      name: "",
      description: "",
      path: "user.created",
    },
  });

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="space-y-5" noValidate>
      <div className="space-y-2">
        <label htmlFor="name" className="text-sm font-medium">
          Name
        </label>
        <input id="name" {...register("name")} className={inputClass} />
        <FieldError message={errors.name?.message} />
      </div>

      <div className="space-y-2">
        <label htmlFor="description" className="text-sm font-medium">
          Description <span className="text-muted-foreground">(optional)</span>
        </label>
        <textarea
          id="description"
          {...register("description")}
          rows={3}
          className={inputClass}
        />
        <FieldError message={errors.description?.message} />
      </div>
      <div className="space-y-2">
        <label htmlFor="path" className="text-sm font-medium">
          Path
        </label>
        <input id="path" {...register("path")} className={inputClass} />
        <FieldError message={errors.path?.message} />
      </div>

      {serverError && (
        <p role="alert" className="text-sm text-red-400">
          {serverError}
        </p>
      )}

      <div className="flex justify-end">
        <button
          type="submit"
          disabled={isSubmitting}
          className="rounded-lg bg-primary px-4 py-2.5 text-sm font-semibold text-background disabled:opacity-50"
        >
          {isSubmitting ? "Creating..." : "Create Topic"}
        </button>
      </div>
    </form>
  );
}

function EditForm({
  initialValues,
  onSubmit,
  isSubmitting,
  serverError,
}: EditProps) {
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<UpdateTopicFormValues>({
    resolver: zodResolver(updateTopicSchema),
    defaultValues: initialValues,
  });

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="space-y-5" noValidate>
      <div className="space-y-2">
        <label htmlFor="name" className="text-sm font-medium">
          Name
        </label>
        <input id="name" {...register("name")} className={inputClass} />
        <FieldError message={errors.name?.message} />
      </div>

      <div className="space-y-2">
        <label htmlFor="description" className="text-sm font-medium">
          Description <span className="text-muted-foreground">(optional)</span>
        </label>
        <textarea
          id="description"
          {...register("description")}
          rows={3}
          className={inputClass}
        />
        <FieldError message={errors.description?.message} />
      </div>
      <label className="flex items-center gap-3 text-sm">
        <input
          type="checkbox"
          {...register("is_active")}
          className="size-4 accent-primary"
        />
        Topic is active
      </label>

      {serverError && (
        <p role="alert" className="text-sm text-red-400">
          {serverError}
        </p>
      )}

      <div className="flex justify-end">
        <button
          type="submit"
          disabled={isSubmitting}
          className="rounded-lg bg-primary px-4 py-2.5 text-sm font-semibold text-background disabled:opacity-50"
        >
          {isSubmitting ? "Saving..." : "Save changes"}
        </button>
      </div>
    </form>
  );
}
