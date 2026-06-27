#![allow(dead_code, unused_variables)]
use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::auth::UserUid;
pub use cloud_object_models::ActionPermission;
pub use cloud_object_models::ComputerUsePermission;
pub use cloud_object_models::AgentModeCommandExecutionPredicate;

pub type WorkspaceUid = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CustomerType { Free, Prosumer, Business, Enterprise, Turbo, Unknown }
impl Default for CustomerType { fn default() -> Self { Self::Free } }
impl CustomerType {
    pub fn to_display_string(&self) -> String {
        match self {
            Self::Free => "Free".to_string(),
            Self::Prosumer => "Prosumer".to_string(),
            Self::Business => "Business".to_string(),
            Self::Enterprise => "Enterprise".to_string(),
            Self::Turbo => "Turbo".to_string(),
            Self::Unknown => "Unknown".to_string(),
        }
    }
}
impl std::fmt::Display for CustomerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_display_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdminEnablementSetting { Enable, Disable, RespectUserSetting }
impl Default for AdminEnablementSetting { fn default() -> Self { Self::RespectUserSetting } }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostEnablementSetting { Enforce, Allow, Deny }
impl Default for HostEnablementSetting { fn default() -> Self { Self::Allow } }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UgcCollectionEnablementSetting { Enable, Disable, RespectUserSetting }
impl Default for UgcCollectionEnablementSetting { fn default() -> Self { Self::RespectUserSetting } }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelinquencyStatus { NoDelinquency, PastDue, TeamLimitExceeded, Unknown, Unpaid }
impl Default for DelinquencyStatus { fn default() -> Self { Self::NoDelinquency } }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ByoApiKeyPolicy { pub enabled: bool }
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnterpriseCreditsAutoReloadPolicy { pub enabled: bool }
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct WorkspaceSizePolicy { pub is_unlimited: bool, pub limit: i64 }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiAutonomySettings {
    pub apply_code_diffs_setting: Option<ActionPermission>,
    pub read_files_setting: Option<ActionPermission>,
    pub read_files_allowlist: Option<Vec<PathBuf>>,
    pub execute_commands_setting: Option<ActionPermission>,
    pub execute_commands_allowlist: Option<Vec<AgentModeCommandExecutionPredicate>>,
    pub execute_commands_denylist: Option<Vec<AgentModeCommandExecutionPredicate>>,
    pub write_to_pty_setting: Option<cloud_object_models::ai_execution_profile::WriteToPtyPermission>,
    pub computer_use_setting: Option<ComputerUsePermission>,
}
impl AiAutonomySettings {
    pub fn has_override_for_code_diffs(&self) -> bool { self.apply_code_diffs_setting.is_some() }
    pub fn has_override_for_read_files(&self) -> bool { self.read_files_setting.is_some() }
    pub fn has_override_for_read_files_allowlist(&self) -> bool { self.read_files_allowlist.is_some() }
    pub fn has_override_for_execute_commands(&self) -> bool { self.execute_commands_setting.is_some() }
    pub fn has_override_for_execute_commands_allowlist(&self) -> bool { self.execute_commands_allowlist.is_some() }
    pub fn has_override_for_execute_commands_denylist(&self) -> bool { self.execute_commands_denylist.is_some() }
    pub fn has_override_for_write_to_pty(&self) -> bool { self.write_to_pty_setting.is_some() }
    pub fn has_override_for_computer_use(&self) -> bool { self.computer_use_setting.is_some() }
    pub fn has_any_overrides(&self) -> bool { false }
}

pub trait AiAutonomySettingsExt {
    fn has_override_for_code_diffs(&self) -> bool;
    fn has_override_for_read_files(&self) -> bool;
    fn has_override_for_read_files_allowlist(&self) -> bool;
    fn has_override_for_execute_commands(&self) -> bool;
    fn has_override_for_execute_commands_allowlist(&self) -> bool;
    fn has_override_for_execute_commands_denylist(&self) -> bool;
    fn has_override_for_write_to_pty(&self) -> bool;
    fn has_override_for_computer_use(&self) -> bool;
    fn has_any_overrides(&self) -> bool;
}
impl AiAutonomySettingsExt for Option<AiAutonomySettings> {
    fn has_override_for_code_diffs(&self) -> bool { self.as_ref().map_or(false, |s| s.has_override_for_code_diffs()) }
    fn has_override_for_read_files(&self) -> bool { self.as_ref().map_or(false, |s| s.has_override_for_read_files()) }
    fn has_override_for_read_files_allowlist(&self) -> bool { self.as_ref().map_or(false, |s| s.has_override_for_read_files_allowlist()) }
    fn has_override_for_execute_commands(&self) -> bool { self.as_ref().map_or(false, |s| s.has_override_for_execute_commands()) }
    fn has_override_for_execute_commands_allowlist(&self) -> bool { self.as_ref().map_or(false, |s| s.has_override_for_execute_commands_allowlist()) }
    fn has_override_for_execute_commands_denylist(&self) -> bool { self.as_ref().map_or(false, |s| s.has_override_for_execute_commands_denylist()) }
    fn has_override_for_write_to_pty(&self) -> bool { self.as_ref().map_or(false, |s| s.has_override_for_write_to_pty()) }
    fn has_override_for_computer_use(&self) -> bool { self.as_ref().map_or(false, |s| s.has_override_for_computer_use()) }
    fn has_any_overrides(&self) -> bool { self.as_ref().map_or(false, |s| s.has_any_overrides()) }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SandboxedAgentSettings { pub execute_commands_denylist: Option<Vec<AgentModeCommandExecutionPredicate>> }
impl SandboxedAgentSettings {
    pub fn has_override_for_code_diffs(&self) -> bool { false }
    pub fn has_override_for_read_files(&self) -> bool { false }
    pub fn has_override_for_read_files_allowlist(&self) -> bool { false }
    pub fn has_override_for_execute_commands(&self) -> bool { false }
    pub fn has_override_for_execute_commands_allowlist(&self) -> bool { false }
    pub fn has_override_for_execute_commands_denylist(&self) -> bool { self.execute_commands_denylist.is_some() }
    pub fn has_override_for_write_to_pty(&self) -> bool { false }
    pub fn has_override_for_computer_use(&self) -> bool { false }
    pub fn has_any_overrides(&self) -> bool { false }
}
pub trait SandboxedAgentSettingsExt {
    fn has_override_for_code_diffs(&self) -> bool;
    fn has_override_for_read_files(&self) -> bool;
    fn has_override_for_read_files_allowlist(&self) -> bool;
    fn has_override_for_execute_commands(&self) -> bool;
    fn has_override_for_execute_commands_allowlist(&self) -> bool;
    fn has_override_for_execute_commands_denylist(&self) -> bool;
    fn has_override_for_write_to_pty(&self) -> bool;
    fn has_override_for_computer_use(&self) -> bool;
    fn has_any_overrides(&self) -> bool;
}
impl SandboxedAgentSettingsExt for Option<SandboxedAgentSettings> {
    fn has_override_for_code_diffs(&self) -> bool { self.as_ref().map_or(false, |s| s.has_override_for_code_diffs()) }
    fn has_override_for_read_files(&self) -> bool { self.as_ref().map_or(false, |s| s.has_override_for_read_files()) }
    fn has_override_for_read_files_allowlist(&self) -> bool { self.as_ref().map_or(false, |s| s.has_override_for_read_files_allowlist()) }
    fn has_override_for_execute_commands(&self) -> bool { self.as_ref().map_or(false, |s| s.has_override_for_execute_commands()) }
    fn has_override_for_execute_commands_allowlist(&self) -> bool { self.as_ref().map_or(false, |s| s.has_override_for_execute_commands_allowlist()) }
    fn has_override_for_execute_commands_denylist(&self) -> bool { self.as_ref().map_or(false, |s| s.has_override_for_execute_commands_denylist()) }
    fn has_override_for_write_to_pty(&self) -> bool { self.as_ref().map_or(false, |s| s.has_override_for_write_to_pty()) }
    fn has_override_for_computer_use(&self) -> bool { self.as_ref().map_or(false, |s| s.has_override_for_computer_use()) }
    fn has_any_overrides(&self) -> bool { self.as_ref().map_or(false, |s| s.has_any_overrides()) }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeminiEnterpriseHostSettings { pub gcp_audience: Option<String>, pub gcp_sa_email: Option<String> }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiOverages { pub current_monthly_request_cost_cents: f64, pub current_monthly_requests_used: u64, pub current_period_end: chrono::DateTime<chrono::Utc> }
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnterpriseSecretRegex { pub name: Option<String>, pub pattern: String }
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct PurchaseAddOnCreditsPolicy { pub enabled: bool }
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct EnterprisePayAsYouGoPolicy { pub enabled: bool }
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct WarpAiPolicy { pub limit: u64 }
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct SharedObjectsPolicy { pub is_unlimited: bool, pub limit: i64 }
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct SessionSharingPolicy { pub max_session_size: u64 }
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AmbientAgentsPolicy { pub enabled: bool, pub instance_shape: Option<AmbientAgentInstanceShape> }
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct AmbientAgentInstanceShape { pub vcpus: u32, pub memory_gb: u32 }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TierSettings {
    pub purchase_add_on_credits_policy: Option<PurchaseAddOnCreditsPolicy>,
    pub enterprise_pay_as_you_go_policy: Option<EnterprisePayAsYouGoPolicy>,
    pub warp_ai_policy: Option<WarpAiPolicy>,
    pub workspace_size_policy: Option<WorkspaceSizePolicy>,
    pub shared_notebooks_policy: Option<SharedObjectsPolicy>,
    pub shared_workflows_policy: Option<SharedObjectsPolicy>,
    pub session_sharing_policy: Option<SessionSharingPolicy>,
    pub ambient_agents_policy: Option<AmbientAgentsPolicy>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmSettings { pub enabled: bool, pub host_configs: HashMap<String, String> }
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AddonCreditsSettings { pub auto_reload_enabled: bool, pub max_monthly_spend_cents: Option<i32>, pub selected_auto_reload_credit_denomination: Option<i32> }
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageBasedPricingSettings { pub enabled: bool, pub max_monthly_spend_cents: Option<u32> }
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceSettings { pub tier: TierSettings, pub addon_credits_settings: AddonCreditsSettings, pub usage_based_pricing_settings: UsageBasedPricingSettings, pub llm_settings: LlmSettings }
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BonusGrantsPurchasedThisMonth { pub cents_spent: i32, pub total_credits_purchased: i32 }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BillingMetadata {
    pub service_agreements: Vec<ServiceAgreement>,
    pub customer_type: CustomerType,
    pub tier: TierSettings,
    pub ai_overages: Option<AiOverages>,
    pub delinquency_status: DelinquencyStatus,
    pub byo_api_key_policy: Option<ByoApiKeyPolicy>,
    pub enterprise_credits_auto_reload_policy: Option<EnterpriseCreditsAutoReloadPolicy>,
    pub workspace_size_policy: Option<WorkspaceSizePolicy>,
}
impl BillingMetadata {
    pub fn is_stripe_paid_plan(_customer_type: CustomerType) -> bool { false }
    pub fn can_upgrade_to_build_plan(&self) -> bool { false }
    pub fn can_upgrade_to_build_max_plan(&self) -> bool { false }
    pub fn can_upgrade_to_higher_tier_plan(&self) -> bool { false }
    pub fn has_overages_used(&self) -> bool { false }
    pub fn has_failed_addon_credit_auto_reload_status(&self) -> bool { false }
    pub fn is_byo_api_key_enabled(&self) -> bool { false }
    pub fn is_delinquent_due_to_payment_issue(&self) -> bool { false }
    pub fn is_free_plan(&self) -> bool { self.customer_type == CustomerType::Free }
    pub fn is_paid_plan(&self) -> bool { !self.is_free_plan() }
    pub fn is_enterprise(&self) -> bool { self.customer_type == CustomerType::Enterprise }
    pub fn overage_limit_cents(&self) -> Option<u64> { None }
    pub fn is_user_on_paid_plan(&self) -> bool { false }
    pub fn is_on_stripe_paid_plan(&self) -> bool { false }
    pub fn is_on_build_plan(&self) -> bool { false }
    pub fn is_on_build_max_plan(&self) -> bool { false }
    pub fn is_on_build_business_plan(&self) -> bool { false }
    pub fn is_on_legacy_paid_plan(&self) -> bool { false }
    pub fn is_on_legacy_business_plan(&self) -> bool { false }
    pub fn is_enterprise_plan(&self) -> bool { self.customer_type == CustomerType::Enterprise }
    pub fn is_enterprise_pay_as_you_go_enabled(&self) -> bool { false }
    pub fn is_enterprise_auto_reload_enabled(&self) -> bool { false }
    pub fn is_purchase_add_on_credits_policy_enabled(&self) -> bool { false }
    pub fn is_usage_based_pricing_toggleable(&self) -> bool { false }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Workspace {
    pub uid: WorkspaceUid,
    pub name: String,
    pub teams: Vec<super::team::Team>,
    pub billing_metadata: BillingMetadata,
    pub settings: WorkspaceSettings,
    pub members: Vec<super::team::TeamMember>,
    pub bonus_grants_purchased_this_month: BonusGrantsPurchasedThisMonth,
}
impl Workspace {
    pub fn from_local_cache<A: 'static, B: 'static, C: 'static>(_a: A, _b: B, _c: C) -> Self { Self::default() }
    pub fn are_overages_enabled(&self) -> bool { false }
    pub fn are_overages_remaining(&self) -> bool { false }
    pub fn are_overages_toggleable(&self) -> bool { false }
    pub fn get_agent_attribution_setting(&self) -> AdminEnablementSetting { AdminEnablementSetting::RespectUserSetting }
    pub fn get_auto_reload_price_cents<T: 'static>(&self, _options: T) -> Option<u64> { None }
    pub fn is_at_addon_credits_monthly_limit(&self) -> bool { false }
    pub fn is_gemini_enterprise_credentials_enabled(&self) -> bool { false }
    pub fn workspaces(&self) -> &[Self] { &[] }
    pub fn would_addon_purchase_reach_limit(&self, _amount: i32) -> bool { false }
    pub fn to_display_string(&self) -> String { self.name.clone() }
}
impl std::fmt::Display for Workspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.name) }
}

#[derive(Debug, Clone, Default)]
pub struct MaybeAiAutonomySettings(pub Option<AiAutonomySettings>);
impl MaybeAiAutonomySettings {
    pub fn has_override_for_code_diffs(&self) -> bool { self.0.as_ref().map_or(false, |s| s.apply_code_diffs_setting.is_some()) }
    pub fn has_override_for_read_files(&self) -> bool { self.0.as_ref().map_or(false, |s| s.read_files_setting.is_some()) }
    pub fn has_override_for_read_files_allowlist(&self) -> bool { self.0.as_ref().map_or(false, |s| s.read_files_allowlist.is_some()) }
    pub fn has_override_for_execute_commands(&self) -> bool { self.0.as_ref().map_or(false, |s| s.execute_commands_setting.is_some()) }
    pub fn has_override_for_execute_commands_allowlist(&self) -> bool { self.0.as_ref().map_or(false, |s| s.execute_commands_allowlist.is_some()) }
    pub fn has_override_for_execute_commands_denylist(&self) -> bool { self.0.as_ref().map_or(false, |s| s.execute_commands_denylist.is_some()) }
    pub fn has_override_for_write_to_pty(&self) -> bool { self.0.as_ref().map_or(false, |s| s.write_to_pty_setting.is_some()) }
    pub fn has_override_for_computer_use(&self) -> bool { self.0.as_ref().map_or(false, |s| s.computer_use_setting.is_some()) }
    pub fn has_any_overrides(&self) -> bool { self.0.is_some() }
}
impl std::ops::Deref for MaybeAiAutonomySettings {
    type Target = Option<AiAutonomySettings>;
    fn deref(&self) -> &Self::Target { &self.0 }
}







#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceAgreement { pub sunsetted_to_build_ts: Option<String> }


