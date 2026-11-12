export type DestinationType = "http";

export interface Destination {
  id: string;
  name: string;
  kind: DestinationType;
  destination_url: string;
  is_active: boolean;
  delivery_timeout_seconds: number;
  created_by: string;
  last_updated_by: string;
  created_at: string;
  updated_at: string;
}

export interface CreateDestinationInput {
  name: string;
  description?: string;
  kind: DestinationType;
  destination_url: string;
  delivery_timeout_seconds: number;
}

export interface UpdateDestinationInput {
  name?: string;
  description?: string;
  kind?: DestinationType;
  destination_url?: string;
  is_active?: boolean;
  delivery_timeout_seconds?: number;
}

export interface DestinationFilters {
  name?: string;
  kind?: DestinationType;
  topic_id?: string;
  linked?: boolean;
}

export interface DestinationListParams extends DestinationFilters {
  page_no: number;
  page_size: number;
}

export interface PageInfo {
  page_size: number;
  page_no: number;
  total_rows: number;
}

export interface PaginatedList<T> {
  list: T[];
  page_info: PageInfo;
}
