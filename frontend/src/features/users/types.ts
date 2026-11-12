
export interface User {
  id: string;
  full_name: string;
  email: string;
  username: string;
  created_at: string;
  updated_at: string;
}

export interface CreateUserInput {
  full_name: string;
  email: string;
  username: string;
  password: string;
}

export interface UpdateUserInput {
  id: string;
  full_name?: string;
}

export interface UserFilters {
  full_name?: string;
  username?: string;
  email?: string;
}

export interface UserListParams extends UserFilters {
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
