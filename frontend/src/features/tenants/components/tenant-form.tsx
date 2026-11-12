import { useEffect } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";

import {
  createTenantSchema,
  updateTenantSchema,
  type CreateTenantFormValues,
  type UpdateTenantFormValues,
} from "@/features/tenants/schemas";

interface CommonProps {
  isSubmitting: boolean;
  serverError: string | null;
}

interface CreateProps extends CommonProps {
  mode: "create";
  onSubmit: (values: CreateTenantFormValues) => Promise<void>;
}

interface EditProps extends CommonProps {
  mode: "edit";
  initialValues: {
    id: string;
    name: string;
    is_active: boolean;
  };
  onSubmit: (values: UpdateTenantFormValues) => Promise<void>;
}

type TenantFormProps = CreateProps | EditProps;

export function TenantForm(props: TenantFormProps) {
  if (props.mode === "create") {
    return <CreateTenantForm {...props} />;
  }

  return <EditTenantForm {...props} />;
}

function CreateTenantForm({
  onSubmit,
  isSubmitting,
  serverError,
}: CreateProps) {
  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<CreateTenantFormValues>({
    resolver: zodResolver(createTenantSchema),
    defaultValues: {
      name: "",
      slug: "",
      default_role_slug: "",
    },
  });

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="space-y-6">
      <div className="space-y-2">
        <label htmlFor="name" className="text-sm font-medium">
          Tenant name
        </label>
        <input
          id="name"
          {...register("name")}
          placeholder="e.g. Acme Corporation"
          className="w-full rounded-lg border border-border bg-background px-3 py-2.5 text-sm outline-none focus:border-primary"
        />
        {errors.name && (
          <p className="text-sm text-red-400">{errors.name.message}</p>
        )}
      </div>

      <div className="space-y-2">
        <label htmlFor="slug" className="text-sm font-medium">
          Tenant slug
        </label>
        <input
          id="slug"
          {...register("slug")}
          placeholder="e.g. acme-corporation"
          className="w-full rounded-lg border border-border bg-background px-3 py-2.5 text-sm outline-none focus:border-primary"
        />
        {errors.slug && (
          <p className="text-sm text-red-400">{errors.slug.message}</p>
        )}
      </div>

      <div className="space-y-2">
        <label htmlFor="default_role_slug" className="text-sm font-medium">
          Default role slug (optional)
        </label>
        <input
          id="default_role_slug"
          {...register("default_role_slug")}
          placeholder="e.g. owner"
          className="w-full rounded-lg border border-border bg-background px-3 py-2.5 text-sm outline-none focus:border-primary"
        />
      </div>

      {serverError && (
        <p className="rounded-lg border border-red-500/30 bg-red-500/10 px-4 py-3 text-sm text-red-300">
          {serverError}
        </p>
      )}

      <SubmitButton isSubmitting={isSubmitting} label="Create tenant" />
    </form>
  );
}

function EditTenantForm({
  initialValues,
  onSubmit,
  isSubmitting,
  serverError,
}: EditProps) {
  const {
    register,
    handleSubmit,
    reset,
    formState: { errors },
  } = useForm<UpdateTenantFormValues>({
    resolver: zodResolver(updateTenantSchema),
    defaultValues: {
      id: initialValues.id,
      name: initialValues.name,
      is_active: initialValues.is_active,
    },
  });

  useEffect(() => {
    reset({
      id: initialValues.id,
      name: initialValues.name,
      is_active: initialValues.is_active,
    });
  }, [initialValues, reset]);

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="space-y-6">
      <div className="space-y-2">
        <label htmlFor="edit-name" className="text-sm font-medium">
          Tenant name
        </label>
        <input
          id="edit-name"
          {...register("name")}
          placeholder="Tenant name"
          className="w-full rounded-lg border border-border bg-background px-3 py-2.5 text-sm outline-none focus:border-primary"
        />
        {errors.name && (
          <p className="text-sm text-red-400">{errors.name.message}</p>
        )}
      </div>

      <label className="flex items-center gap-3 rounded-lg border border-border bg-card p-4">
        <input
          type="checkbox"
          {...register("is_active")}
          className="size-4 accent-primary"
        />
        <span>
          <span className="block text-sm font-medium">Tenant active</span>
          <span className="block text-xs text-muted-foreground">
            Control whether this tenant is active.
          </span>
        </span>
      </label>

      {serverError && (
        <p className="rounded-lg border border-red-500/30 bg-red-500/10 px-4 py-3 text-sm text-red-300">
          {serverError}
        </p>
      )}

      <SubmitButton isSubmitting={isSubmitting} label="Save changes" />
    </form>
  );
}

function SubmitButton({
  isSubmitting,
  label,
}: {
  isSubmitting: boolean;
  label: string;
}) {
  return (
    <button
      type="submit"
      disabled={isSubmitting}
      className="rounded-lg bg-primary px-4 py-2.5 text-sm font-semibold text-primary-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
    >
      {isSubmitting ? "Saving..." : label}
    </button>
  );
}
