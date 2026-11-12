
import { z } from "zod";

export const createTenantSchema = z.object({
  name: z
    .string()
    .trim()
    .min(1, "Tenant name is required"),

  slug: z
    .string()
    .trim()
    .min(1, "Tenant slug is required")
    .regex(
      /^[a-z0-9]+(?:-[a-z0-9]+)*$/,
      "Use lowercase letters, numbers, and hyphens",
    ),

  default_role_slug: z
    .string()
    .trim()
    .optional()
    .or(z.literal("")),
});

export const updateTenantSchema = z.object({
  id: z.string().uuid("Invalid tenant ID"),
  name: z
    .string()
    .trim()
    .min(1, "Tenant name is required")
    .optional(),
  is_active: z.boolean().optional(),
});

export type CreateTenantFormValues = z.infer<
  typeof createTenantSchema
>;

export type UpdateTenantFormValues = z.infer<
  typeof updateTenantSchema
>;
