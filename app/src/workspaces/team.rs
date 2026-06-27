#![allow(dead_code, unused_variables)]
use serde::{Deserialize, Serialize};
use crate::auth::UserUid;
use crate::server::ids::ServerId;
use super::workspace::{BillingMetadata, WorkspaceUid};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MembershipRole { Owner, Admin, User }
impl Default for MembershipRole { fn default() -> Self { Self::User } }
impl std::fmt::Display for MembershipRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{:?}", self) }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeamDeleteDisabledReason { ActivePaidSubscription }
impl TeamDeleteDisabledReason { pub fn user_facing_message(&self) -> &str { "" } }
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrganizationSettings { pub is_invite_link_enabled: bool, pub is_discoverable: bool }
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemberUsageInfo { pub requests_used_since_last_refresh: u64, pub is_unlimited: bool, pub request_limit: u64, pub is_request_limit_prorated: bool }
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PendingEmailInvite { pub invitee_email: String, pub expired: bool }
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DomainRestriction { pub domain: String, pub uid: ServerId }
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InviteCode { pub code: String, pub uid: String }
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TeamMember { pub uid: UserUid, pub role: MembershipRole, pub display_name: Option<String>, pub email: String, pub photo_url: Option<String>, pub usage_info: MemberUsageInfo }
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Team {
    pub has_billing_history: bool,
    pub uid: ServerId, pub name: String, pub members: Vec<TeamMember>, pub billing_metadata: BillingMetadata,
    pub discoverable: bool, pub invite_code: Option<InviteCode>, pub invite_link_domain_restrictions: Vec<DomainRestriction>,
    pub pending_email_invites: Vec<PendingEmailInvite>, pub organization_settings: OrganizationSettings, pub is_eligible_for_discovery: bool,
}
pub type TeamMetadata = Team;
impl Team {
    pub fn from_local_cache<A: 'static, B: 'static, C: 'static, D: 'static, E: 'static>(_a: A, _b: B, _c: C, _d: D, _e: E) -> Self { Self::default() }
    pub fn get_delete_disabled_reason(&self, _email: &str, _remaining_credits: i32) -> Option<TeamDeleteDisabledReason> { None }
    pub fn has_admin_permissions(&self, _email: &str) -> bool { false }
    pub fn has_owner_permissions(&self, _email: &str) -> bool { false }
    pub fn is_custom_llm_enabled(&self) -> bool { false }
    pub fn is_multi_admin_enabled(&self) -> bool { false }
    pub fn user_facing_message(&self) -> Option<&str> { None }
    pub fn create_team<A: 'static, B: 'static, C: 'static>(&mut self, _a: A, _b: B, _c: C) {}
    pub fn leave_team<A: 'static, B: 'static>(&mut self, _a: A, _b: B) {}
    pub fn rename_team<A: 'static, B: 'static>(&mut self, _a: A, _b: B) {}
    pub fn stop_polling_for_workspace_metadata_updates(&mut self) {}
    pub fn num_members(&self) -> i64 { self.members.len() as i64 }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiscoverableTeam { pub uid: WorkspaceUid, pub team_uid: WorkspaceUid, pub name: String, pub member_count: isize, pub num_members: isize, pub team_accepting_invites: bool }