
export interface LoginRequest {
  username: string;
  password: string;
}

export interface LoginResponse {
  username: string;
  email: string;
  token: string;
}

export interface MembershipDetails {
  tenant_id: string;
  tenant_name: string;
  tenant_slug: string;
  role_id: string;
  role_title: string;
  role_slug: string;
}

export interface UserInfo {
  is_admin: boolean;
  username: string;
  email: string;
  memberships: MembershipDetails[];
}
