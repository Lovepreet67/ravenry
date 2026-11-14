
import { z } from "zod";

export const createDestinationSchema = z.object({
  name: z.string().trim().min(1, "Name is required"),
  description: z.string().optional(),
  kind: z.enum(["http"]),
  destination_url: z.string().trim().url("Enter a valid URL"),
  delivery_timeout_seconds: z.coerce
    .number()
    .int("Timeout must be a whole number")
    .min(1, "Timeout must be at least 1 second"),
  retry_delay_ms: z.coerce
    .number()
    .int("retry delay must be a whole number"),
  max_attempts: z.coerce
    .number()
    .int("Max attempts must be a whole number")
    .min(1, "Attempts must be at least 1 second"),
});

export const updateDestinationSchema = z.object({
  name: z.string().trim().min(1, "Name is required"),
  kind: z.enum(["http"]),
  destination_url: z.string().trim().url("Enter a valid URL"),
  delivery_timeout_seconds: z.coerce
    .number()
    .int("Timeout must be a whole number")
    .min(1, "Timeout must be at least 1 second"),
  retry_delay_ms: z.coerce
    .number()
    .int("retry delay must be a whole number"),
  max_attempts: z.coerce
    .number()
    .int("Max attempts must be a whole number")
    .min(1, "Attempts must be at least 1 second"),
  is_active: z.boolean(),

});

export type CreateDestinationFormValues = z.infer<
  typeof createDestinationSchema
>;

export type UpdateDestinationFormValues = z.infer<
  typeof updateDestinationSchema
>;
