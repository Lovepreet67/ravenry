import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import {
  updateUserSchema,
  type UpdateUserFormValues,
} from "@/features/users/schemas";

interface EditUserFormProps {
  initialValues: UpdateUserFormValues;
  onSubmit: (values: UpdateUserFormValues) => Promise<void>;
  isSubmitting: boolean;
  serverError: string | null;
}

export function EditUserForm({
  initialValues,
  onSubmit,
  isSubmitting,
  serverError,
}: EditUserFormProps) {
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<UpdateUserFormValues>({
    resolver: zodResolver(updateUserSchema),
    defaultValues: initialValues,
  });

  const inputClass =
    "w-full rounded-lg border border-border bg-background px-3 py-2.5 text-sm outline-none transition-colors focus:border-primary";

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="space-y-5" noValidate>
      <div className="space-y-2">
        <label htmlFor="full_name" className="text-sm font-medium">
          Full name
        </label>
        <input
          id="full_name"
          {...register("full_name")}
          className={inputClass}
          autoComplete="name"
          aria-invalid={!!errors.full_name}
        />
        {errors.full_name && (
          <p className="text-sm text-red-400">{errors.full_name.message}</p>
        )}
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
          className="rounded-lg bg-primary px-4 py-2.5 text-sm font-semibold text-background transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
        >
          {isSubmitting ? "Saving changes..." : "Save changes"}
        </button>
      </div>
    </form>
  );
}
