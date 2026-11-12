import { z } from "zod";

export const createTopicSchema = z.object({
  name: z.string().trim().min(1, "Name is required"),
  description: z.string().optional(),
  path: z.string().trim().min(1, "Path is required"),
});

export const updateTopicSchema = z.object({
  name: z.string().trim().min(1, "Name is required"),
  description: z.string().optional(),
  is_active: z.boolean(),
});

export type CreateTopicFormValues = z.infer<typeof createTopicSchema>;

export type UpdateTopicFormValues = z.infer<typeof updateTopicSchema>;
