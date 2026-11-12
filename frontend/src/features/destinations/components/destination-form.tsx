import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";

import {
  createDestinationSchema,
  updateDestinationSchema,
  type CreateDestinationFormValues,
  type UpdateDestinationFormValues,
} from "@/features/destinations/schemas";

interface CommonProps {
  isSubmitting: boolean;
  serverError: string | null;
}

interface CreateProps extends CommonProps {
  mode: "create";
  onSubmit: (values: CreateDestinationFormValues) => Promise<void>;
}

interface EditProps extends CommonProps {
  mode: "edit";
  initialValues: UpdateDestinationFormValues;
  onSubmit: (values: UpdateDestinationFormValues) => Promise<void>;
}

type Props = CreateProps | EditProps;

const inputClass =
  "w-full rounded-lg border border-border bg-background px-3 py-2.5 text-sm outline-none focus:border-primary";

function FieldError({ message }: { message?: string }) {
  if (!message) return null;

  return <p className="text-sm text-red-400">{message}</p>;
}

export function DestinationForm(props: Props) {
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
  } = useForm<CreateDestinationFormValues>({
    resolver: zodResolver(createDestinationSchema),
    defaultValues: {
      name: "",
      description: "",
      kind: "http",
      destination_url: "",
      delivery_timeout_seconds: 30,
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
        <label htmlFor="kind" className="text-sm font-medium">
          Type
        </label>
        <select id="kind" {...register("kind")} className={inputClass}>
          <option value="http">HTTP</option>
        </select>
      </div>

      <div className="space-y-2">
        <label htmlFor="destination_url" className="text-sm font-medium">
          Destination URL
        </label>
        <input
          id="destination_url"
          type="url"
          {...register("destination_url")}
          placeholder="https://example.com/webhook"
          className={inputClass}
        />
        <FieldError message={errors.destination_url?.message} />
      </div>

      <div className="space-y-2">
        <label
          htmlFor="delivery_timeout_seconds"
          className="text-sm font-medium"
        >
          Delivery timeout (seconds)
        </label>
        <input
          id="delivery_timeout_seconds"
          type="number"
          min={1}
          {...register("delivery_timeout_seconds")}
          className={inputClass}
        />
        <FieldError message={errors.delivery_timeout_seconds?.message} />
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
          {isSubmitting ? "Creating..." : "Create destination"}
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
  } = useForm<UpdateDestinationFormValues>({
    resolver: zodResolver(updateDestinationSchema),
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
        <label htmlFor="kind" className="text-sm font-medium">
          Type
        </label>
        <select id="kind" {...register("kind")} className={inputClass}>
          <option value="http">HTTP</option>
        </select>
      </div>

      <div className="space-y-2">
        <label htmlFor="destination_url" className="text-sm font-medium">
          Destination URL
        </label>
        <input
          id="destination_url"
          type="url"
          {...register("destination_url")}
          className={inputClass}
        />
        <FieldError message={errors.destination_url?.message} />
      </div>

      <div className="space-y-2">
        <label
          htmlFor="delivery_timeout_seconds"
          className="text-sm font-medium"
        >
          Delivery timeout (seconds)
        </label>
        <input
          id="delivery_timeout_seconds"
          type="number"
          min={1}
          {...register("delivery_timeout_seconds")}
          className={inputClass}
        />
        <FieldError message={errors.delivery_timeout_seconds?.message} />
      </div>

      <label className="flex items-center gap-3 text-sm">
        <input
          type="checkbox"
          {...register("is_active")}
          className="size-4 accent-primary"
        />
        Destination is active
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
