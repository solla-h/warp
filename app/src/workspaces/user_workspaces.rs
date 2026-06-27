#![allow(dead_code, unused_variables)]
use cloud_objects::cloud_object::Owner;
use warpui::{AppContext, Entity, ModelContext, ModelHandle, SingletonEntity};
use crate::auth::UserUid;
use crate::server::ids::ServerId;
use super::team::{DiscoverableTeam, MembershipRole, Team, TeamMember};
use super::workspace::*;

#[derive(Debug, Clone)]
pub enum UserWorkspacesEvent {
    TeamsChanged, AiOveragesUpdated, CodebaseContextEnablementChanged, SunsettedToBuildDataUpdated,
    AddDomainRestrictionsRejected(String), AddDomainRestrictionsSuccess,
    DeleteDomainRestrictionRejected(String), DeleteDomainRestrictionSuccess,
    DeleteTeamInvite, DeleteTeamInviteRejected(String),
    EmailInviteRejected(String), EmailInviteSent,
    FetchDiscoverableTeamsRejected(String), FetchDiscoverableTeamsSuccess(Vec<DiscoverableTeam>),
    GenerateStripeBillingPortalLink(String), GenerateStripeBillingPortalLinkRejected(String),
    GenerateUpgradeLink(String), GenerateUpgradeLinkRejected(String),
    JoinTeamWithTeamDiscoveryRejected(String), JoinTeamWithTeamDiscoverySuccess,
    PurchaseAddonCreditsRejected(String), PurchaseAddonCreditsSuccess,
    ResetInviteLinks, ResetInviteLinksRejected(String),
    SetTeamMemberRoleRejected(String), SetTeamMemberRoleSuccess,
    ToggleInviteLinksRejected(String), ToggleInviteLinksSuccess,
    ToggleTeamDiscoverabilityRejected(String), ToggleTeamDiscoverabilitySuccess,
    TransferTeamOwnershipRejected(String), TransferTeamOwnershipSuccess,
    UpdateWorkspaceSettingsRejected(String), UpdateWorkspaceSettingsSuccess,
}
#[derive(Debug, Clone, Default)]
pub struct WorkspacesMetadataResponse;
#[derive(Debug, Clone, Default)]
pub struct WorkspacesMetadataWithPricing;
#[derive(Debug, Clone)]
pub struct CreateTeamResponse { pub team_uid: String }
#[derive(Default)]
pub struct UserWorkspaces;
impl UserWorkspaces {
    pub fn new<A: 'static, B: 'static, C: 'static, D: 'static>(_a: A, _b: B, _c: C, _d: D, _ctx: &mut ModelContext<Self>) -> Self { Self }
    pub fn mock(_ctx: &mut ModelContext<Self>) -> Self { Self }
    pub fn default_mock(_ctx: &mut ModelContext<Self>) -> Self { Self }
    pub fn update_workspaces<R: 'static>(&mut self, _response: R, _ctx: &mut ModelContext<Self>) {}
    pub fn upgrade_link(_user_uid: UserUid) -> String { String::new() }
    pub fn upgrade_link_for_team(_team_uid: ServerId) -> String { String::new() }
    pub fn personal_drive(&self, _ctx: &AppContext) -> Option<Owner> { None }
    pub fn has_capacity_for_shared_notebooks(_team_uid: ServerId, _ctx: &AppContext, _count: usize) -> bool { true }
    pub fn has_capacity_for_shared_workflows(_team_uid: ServerId, _ctx: &AppContext, _count: usize) -> bool { true }
    pub fn is_at_tier_limit_for_object_type<T: 'static>(_object_type: T, _ctx: &AppContext) -> bool { false }
    pub fn is_gemini_enterprise_credentials_enabled(&self, _ctx: &AppContext) -> bool { false }
    pub fn get_agent_attribution_setting(&self) -> AdminEnablementSetting { AdminEnablementSetting::RespectUserSetting }
    pub fn current_team(&self) -> Option<&Team> { None }
    pub fn current_team_uid(&self) -> Option<ServerId> { None }
    pub fn current_workspace(&self) -> Option<&Workspace> { None }
    pub fn default_host_slug(&self) -> Option<&str> { None }
    pub fn has_teams(&self) -> bool { false }
    pub fn has_workspaces(&self) -> bool { false }
    pub fn user_email(&self) -> &str { "" }
    pub fn team_spaces(&self) -> Vec<crate::cloud_object::Space> { vec![] }
    pub fn all_user_spaces(&self, _ctx: &AppContext) -> Vec<crate::cloud_object::Space> { vec![] }
    pub fn team_from_uid(&self, _uid: ServerId) -> Option<&Team> { None }
    pub fn team_from_uid_across_all_workspaces<U: 'static>(&self, _uid: U) -> Option<&Team> { None }
    pub fn num_joinable_teams(&self) -> usize { 0 }
    pub fn total_teammates_in_joinable_teams(&self) -> usize { 0 }
    pub fn owner_to_space(&self, _owner: impl std::borrow::Borrow<Owner>, _ctx: &AppContext) -> Space { Space::Personal }
    pub fn space_to_owner(&self, _space: impl std::borrow::Borrow<crate::cloud_object::Space>, _ctx: &AppContext) -> Option<Owner> { None }
    pub fn workspaces_metadata(&self) -> Option<&WorkspacesMetadataResponse> { None }
    pub fn workspaces(&self) -> &[Workspace] { &[] }
    pub fn usage_based_pricing_settings(&self) -> UsageBasedPricingSettings { UsageBasedPricingSettings::default() }
    pub fn show_code_suggestion_speedbump(&self) -> bool { false }
    pub fn is_ai_allowed_in_remote_sessions(&self) -> bool { true }
    pub fn is_ai_autonomy_allowed(&self) -> bool { true }
    pub fn is_any_ai_enabled(&self, _ctx: &AppContext) -> bool { true }
    pub fn is_anyone_with_link_sharing_enabled(&self) -> bool { false }
    pub fn is_aws_bedrock_available_from_workspace(&self) -> bool { false }
    pub fn is_aws_bedrock_credentials_enabled(&self, _ctx: &AppContext) -> bool { false }
    pub fn is_aws_bedrock_credentials_toggleable(&self) -> bool { false }
    pub fn is_byo_api_key_enabled(&self, _ctx: &AppContext) -> bool { true }
    pub fn is_code_suggestions_toggleable(&self) -> bool { true }
    pub fn is_codebase_context_enabled(&self, _ctx: &AppContext) -> bool { true }
    pub fn is_custom_inference_enabled(&self, _ctx: &AppContext) -> bool { false }
    pub fn is_delinquent_due_to_payment_issue(&self) -> bool { false }
    pub fn is_direct_link_sharing_enabled(&self) -> bool { false }
    pub fn is_enterprise_secret_redaction_enabled(&self) -> bool { false }
    pub fn is_git_operations_ai_enabled(&self) -> bool { true }
    pub fn is_next_command_enabled(&self) -> bool { true }
    pub fn is_prompt_suggestions_toggleable(&self) -> bool { true }
    pub fn is_voice_enabled(&self) -> bool { true }
    pub fn is_voice_input_enabled(&self) -> bool { true }
    pub fn ai_allowed_for_current_team(&self) -> bool { true }
    pub fn team_allows_codebase_context(&self) -> AdminEnablementSetting { AdminEnablementSetting::RespectUserSetting }
    pub fn can_upgrade_to_build_plan(&self) -> bool { false }
    pub fn can_upgrade_to_higher_tier_plan(&self) -> bool { false }
    pub fn ai_autonomy_settings(&self) -> AiAutonomySettings { AiAutonomySettings::default() }
    pub fn sandboxed_agent_settings(&self) -> Option<SandboxedAgentSettings> { None }
    pub fn aws_bedrock_host_enablement_setting(&self) -> HostEnablementSetting { HostEnablementSetting::Allow }
    pub fn gemini_enterprise_host_settings(&self) -> Option<GeminiEnterpriseHostSettings> { None }
    pub fn get_cloud_conversation_storage_enablement_setting(&self) -> AdminEnablementSetting { AdminEnablementSetting::RespectUserSetting }
    pub fn get_enterprise_secret_redaction_regex_list(&self) -> Vec<EnterpriseSecretRegex> { vec![] }
    pub fn get_remote_session_regex_list(&self) -> Vec<regex::Regex> { vec![] }
    pub fn get_ugc_collection_enablement_setting(&self) -> UgcCollectionEnablementSetting { UgcCollectionEnablementSetting::RespectUserSetting }
    pub fn api_keys_for_request(&self) -> Option<()> { None }
    pub fn add_invite_link_domain_restrictions<T: 'static>(&mut self, _team_uid: T, _domains: Vec<String>, _ctx: &mut ModelContext<Self>) {}
    pub fn delete_invite_link_domain_restriction<T: 'static, D: 'static>(&mut self, _team_uid: T, _domain: D, _ctx: &mut ModelContext<Self>) {}
    pub fn delete_team_invite<T: 'static, D: 'static>(&mut self, _team_uid: T, _email: D, _ctx: &mut ModelContext<Self>) {}
    pub fn fetch_discoverable_teams(&mut self, _ctx: &mut ModelContext<Self>) {}
    pub fn generate_stripe_billing_portal_link<T: 'static>(&mut self, _team_uid: T, _ctx: &mut ModelContext<Self>) {}
    pub fn generate_upgrade_link<T: 'static>(&mut self, _team_uid: T, _ctx: &mut ModelContext<Self>) {}
    pub fn join_team_with_team_discovery<T: 'static>(&mut self, _team_uid: T, _ctx: &mut ModelContext<Self>) {}
    pub fn purchase_addon_credits<A: 'static, B: 'static>(&mut self, _team_uid: A, _amount: B, _ctx: &mut ModelContext<Self>) {}
    pub fn refresh_ai_overages(&mut self, _ctx: &mut ModelContext<Self>) {}
    pub fn remove_aws_bedrock_login_banner(&mut self) {}
    pub fn remove_user_from_team<A: 'static, B: 'static, C: 'static>(&mut self, _a: A, _b: B, _c: C, _ctx: &mut ModelContext<Self>) {}
    pub fn reset_invite_links<T: 'static>(&mut self, _team_uid: T, _ctx: &mut ModelContext<Self>) {}
    pub fn send_email_invites<T: 'static>(&mut self, _team_uid: T, _emails: Vec<String>, _ctx: &mut ModelContext<Self>) {}
    pub fn set_current_workspace_uid(&mut self, _uid: &str) {}
    pub fn set_grok_refresh_allowed(&mut self, _allowed: bool) {}
    pub fn set_is_invite_link_enabled<T: 'static>(&mut self, _team_uid: T, _enabled: bool, _ctx: &mut ModelContext<Self>) {}
    pub fn set_team_discoverability<T: 'static>(&mut self, _team_uid: T, _discoverable: bool, _ctx: &mut ModelContext<Self>) {}
    pub fn set_team_member_role<A: 'static, B: 'static>(&mut self, _user_uid: A, _team_uid: B, _role: MembershipRole, _ctx: &mut ModelContext<Self>) {}
    pub fn setup_test_workspace(&mut self, _ctx: &mut ModelContext<Self>) {}
    pub fn update<R: 'static>(&mut self, _response: R, _ctx: &mut ModelContext<Self>) {}
    pub fn update_addon_credits_settings(&mut self, _team_uid: crate::server::ids::ServerId, _enabled: Option<bool>, _monthly_limit: Option<i32>, _credits: Option<i32>, _ctx: &mut ModelContext<Self>) {}
    pub fn update_current_workspace(&mut self, _workspace: Workspace, _ctx: &mut ModelContext<Self>) {}
    pub fn update_object_location(&mut self, _ctx: &mut ModelContext<Self>) {}
    pub fn update_session_team_permissions(&mut self, _ctx: &mut ModelContext<Self>) {}
    pub fn update_usage_based_pricing_settings<U: 'static, V: 'static, W: 'static>(&mut self, _a: U, _b: V, _c: W, _ctx: &mut ModelContext<Self>) {}
    pub fn update_warp_drive_view(&mut self, _ctx: &mut ModelContext<Self>) {}
    pub fn transfer_team_ownership<A: 'static, B: 'static>(&mut self, _team_uid: A, _user_uid: B, _ctx: &mut ModelContext<Self>) {}
}
impl Entity for UserWorkspaces { type Event = UserWorkspacesEvent; }
impl SingletonEntity for UserWorkspaces {}

use crate::cloud_object::Space;