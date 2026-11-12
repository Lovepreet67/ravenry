
export interface MembershipDetails {
  tenant_id: string;
  tenant_name: string;
  tenant_slug: string;
  role_id: string;
  role_title: string;
  role_slug: string;
}

export interface UserMemberships {
  user_id: string;
  memberships: MembershipDetails[];
}

export interface UpsertMembershipInput {
  user_id: string;
  role_id: string;
}
