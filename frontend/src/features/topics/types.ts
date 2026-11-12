export interface Topic {
  id: string;
  name: string;
  description?: string;
  path: string;
  is_active: boolean;
  created_by: string;
  last_updated_by: string;
  created_at: string;
  updated_at: string;
}

export interface CreateTopicInput {
  name: string;
  description?: string;
  path: string;
}

export interface UpdateTopicInput {
  name?: string;
  description?: string;
  is_active?: boolean;
}

export interface TopicFilters {
  name?: string;
}

export interface TopicListParams extends TopicFilters {
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
