export interface CursorPage<T> {
  items: T[];
  next_cursor: string | null;
}

export interface EventItem {
  event_id: string;
  topic_id: string;
  topic_path: string;
  schema_version: number;
  received_at: string;
}

export interface Delivery {
  delivery_id: string;
  event_id: string;
  topic_id: string;
  destination_id: string;
  status: string;
  current_attempt: number;
  created_at: string;
  updated_at: string;
}

export interface DeliveryAttempt {
  attempt_id: string;
  attempt_number: number;
  success: boolean;
  status_code: number | null;
  error: string | null;
  attempted_at: string;
}

export interface DeliveryAttemptsResponse {
  delivery: Delivery;
  attempts: DeliveryAttempt[];
  next_cursor: string | null;
}
