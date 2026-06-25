use anyhow::Result;
use async_trait::async_trait;
#[cfg(test)]
use mockall::{automock, predicate::*};

use crate::server::ids::ServerId;
use crate::workspaces::user_workspaces::WorkspacesMetadataResponse;
use crate::workspaces::workspace::AiOverages;

#[cfg_attr(test, automock)]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait WorkspaceClient: 'static + Send + Sync {
    async fn generate_stripe_billing_portal_link(&self, team_uid: ServerId) -> Result<String>;

    async fn update_usage_based_pricing_settings(
        &self,
        team_uid: ServerId,
        usage_based_pricing_enabled: bool,
        max_monthly_spend_cents: Option<u32>,
    ) -> Result<WorkspacesMetadataResponse>;

    async fn refresh_ai_overages(&self) -> Result<AiOverages>;

    async fn purchase_addon_credits(
        &self,
        team_uid: ServerId,
        credits: i32,
    ) -> Result<WorkspacesMetadataResponse>;

    async fn update_addon_credits_settings(
        &self,
        team_uid: ServerId,
        auto_reload_enabled: Option<bool>,
        max_monthly_spend_cents: Option<i32>,
        selected_auto_reload_credit_denomination: Option<i32>,
    ) -> Result<WorkspacesMetadataResponse>;
}

use super::ServerApi;

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl WorkspaceClient for ServerApi {
    async fn generate_stripe_billing_portal_link(&self, _team_uid: ServerId) -> Result<String> {
        todo!("GraphQL backend removed")
    }

    async fn update_usage_based_pricing_settings(
        &self,
        _team_uid: ServerId,
        _usage_based_pricing_enabled: bool,
        _max_monthly_spend_cents: Option<u32>,
    ) -> Result<WorkspacesMetadataResponse> {
        todo!("GraphQL backend removed")
    }

    async fn refresh_ai_overages(&self) -> Result<AiOverages> {
        todo!("GraphQL backend removed")
    }

    async fn purchase_addon_credits(
        &self,
        _team_uid: ServerId,
        _credits: i32,
    ) -> Result<WorkspacesMetadataResponse> {
        todo!("GraphQL backend removed")
    }

    async fn update_addon_credits_settings(
        &self,
        _team_uid: ServerId,
        _auto_reload_enabled: Option<bool>,
        _max_monthly_spend_cents: Option<i32>,
        _selected_auto_reload_credit_denomination: Option<i32>,
    ) -> Result<WorkspacesMetadataResponse> {
        todo!("GraphQL backend removed")
    }
}

