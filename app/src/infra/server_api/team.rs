use anyhow::Result;
use async_trait::async_trait;
#[cfg(test)]
use mockall::{automock, predicate::*};

use crate::auth::UserUid;
use crate::cloud_object::CloudObjectEventEntrypoint;
use crate::ids::ServerId;
use crate::workspaces::team::{DiscoverableTeam, MembershipRole};
use crate::workspaces::user_workspaces::{CreateTeamResponse, WorkspacesMetadataWithPricing};

#[cfg_attr(test, automock)]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait TeamClient: 'static + Send + Sync {
    async fn workspaces_metadata(&self) -> Result<WorkspacesMetadataWithPricing>;

    async fn add_invite_link_domain_restriction(
        &self,
        team_uid: ServerId,
        domain: String,
    ) -> Result<WorkspacesMetadataWithPricing>;

    async fn delete_invite_link_domain_restriction(
        &self,
        team_uid: ServerId,
        domain_uid: ServerId,
    ) -> Result<WorkspacesMetadataWithPricing>;

    /// Creates a team and returns the result from the server with the newly created team.
    async fn create_team(
        &self,
        name: String,
        entrypoint: CloudObjectEventEntrypoint,
        discoverable: Option<bool>,
    ) -> Result<CreateTeamResponse>;

    /// Removes the user from the selected team and returns a list of all teams that a user is
    /// still a member of (including updated team members).
    async fn remove_user_from_team(
        &self,
        user_uid: UserUid,
        team_uid: ServerId,
        entrypoint: CloudObjectEventEntrypoint,
    ) -> Result<WorkspacesMetadataWithPricing>;

    /// Removes the _current_ user from the team (user leaving the team) and returns the list of
    /// all teams that the current user is still a member of.
    async fn leave_team(
        &self,
        user_uid: UserUid,
        team_uid: ServerId,
        entrypoint: CloudObjectEventEntrypoint,
    ) -> Result<WorkspacesMetadataWithPricing>;

    async fn join_team_with_team_discovery(
        &self,
        team_uid: ServerId,
    ) -> Result<WorkspacesMetadataWithPricing>;

    async fn send_team_invite_email(
        &self,
        team_uid: ServerId,
        email: String,
    ) -> Result<WorkspacesMetadataWithPricing>;

    async fn delete_team_invite(
        &self,
        team_uid: ServerId,
        email: String,
    ) -> Result<WorkspacesMetadataWithPricing>;

    async fn get_discoverable_teams(&self) -> Result<Vec<DiscoverableTeam>>;

    async fn rename_team(
        &self,
        new_name: String,
        team_uid: ServerId,
    ) -> Result<WorkspacesMetadataWithPricing>;

    async fn reset_invite_links(&self, team_uid: ServerId)
        -> Result<WorkspacesMetadataWithPricing>;

    async fn set_is_invite_link_enabled(
        &self,
        team_uid: ServerId,
        new_value: bool,
    ) -> Result<WorkspacesMetadataWithPricing>;

    async fn set_team_discoverability(
        &self,
        team_uid: ServerId,
        discoverable: bool,
    ) -> Result<WorkspacesMetadataWithPricing>;

    async fn transfer_team_ownership(
        &self,
        new_owner_email: String,
    ) -> Result<WorkspacesMetadataWithPricing>;

    async fn set_team_member_role(
        &self,
        user_uid: UserUid,
        team_uid: ServerId,
        role: MembershipRole,
    ) -> Result<WorkspacesMetadataWithPricing>;
}

use super::ServerApi;

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl TeamClient for ServerApi {
    async fn workspaces_metadata(&self) -> Result<WorkspacesMetadataWithPricing> {
        todo!("GraphQL backend removed")
    }

    async fn add_invite_link_domain_restriction(
        &self,
        _team_uid: ServerId,
        _domain: String,
    ) -> Result<WorkspacesMetadataWithPricing> {
        todo!("GraphQL backend removed")
    }

    async fn delete_invite_link_domain_restriction(
        &self,
        _team_uid: ServerId,
        _domain_uid: ServerId,
    ) -> Result<WorkspacesMetadataWithPricing> {
        todo!("GraphQL backend removed")
    }

    async fn create_team(
        &self,
        _name: String,
        _entrypoint: CloudObjectEventEntrypoint,
        _discoverable: Option<bool>,
    ) -> Result<CreateTeamResponse> {
        todo!("GraphQL backend removed")
    }

    async fn remove_user_from_team(
        &self,
        _user_uid: UserUid,
        _team_uid: ServerId,
        _entrypoint: CloudObjectEventEntrypoint,
    ) -> Result<WorkspacesMetadataWithPricing> {
        todo!("GraphQL backend removed")
    }

    async fn leave_team(
        &self,
        _user_uid: UserUid,
        _team_uid: ServerId,
        _entrypoint: CloudObjectEventEntrypoint,
    ) -> Result<WorkspacesMetadataWithPricing> {
        todo!("GraphQL backend removed")
    }

    async fn join_team_with_team_discovery(
        &self,
        _team_uid: ServerId,
    ) -> Result<WorkspacesMetadataWithPricing> {
        todo!("GraphQL backend removed")
    }

    async fn send_team_invite_email(
        &self,
        _team_uid: ServerId,
        _email: String,
    ) -> Result<WorkspacesMetadataWithPricing> {
        todo!("GraphQL backend removed")
    }

    async fn delete_team_invite(
        &self,
        _team_uid: ServerId,
        _email: String,
    ) -> Result<WorkspacesMetadataWithPricing> {
        todo!("GraphQL backend removed")
    }

    async fn get_discoverable_teams(&self) -> Result<Vec<DiscoverableTeam>> {
        todo!("GraphQL backend removed")
    }

    async fn rename_team(
        &self,
        _new_name: String,
        _team_uid: ServerId,
    ) -> Result<WorkspacesMetadataWithPricing> {
        todo!("GraphQL backend removed")
    }

    async fn reset_invite_links(
        &self,
        _team_uid: ServerId,
    ) -> Result<WorkspacesMetadataWithPricing> {
        todo!("GraphQL backend removed")
    }

    async fn set_is_invite_link_enabled(
        &self,
        _team_uid: ServerId,
        _new_value: bool,
    ) -> Result<WorkspacesMetadataWithPricing> {
        todo!("GraphQL backend removed")
    }

    async fn set_team_discoverability(
        &self,
        _team_uid: ServerId,
        _discoverable: bool,
    ) -> Result<WorkspacesMetadataWithPricing> {
        todo!("GraphQL backend removed")
    }

    async fn transfer_team_ownership(
        &self,
        _new_owner_email: String,
    ) -> Result<WorkspacesMetadataWithPricing> {
        todo!("GraphQL backend removed")
    }

    async fn set_team_member_role(
        &self,
        _user_uid: UserUid,
        _team_uid: ServerId,
        _role: MembershipRole,
    ) -> Result<WorkspacesMetadataWithPricing> {
        todo!("GraphQL backend removed")
    }
}

