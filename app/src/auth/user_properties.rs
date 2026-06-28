use super::user::User;
use super::UserUid;
use crate::server_experiments::ServerExperiment;

/// Intermediate app model state converted from a user response returned by the auth client.
pub(crate) struct UserProperties {
    pub(crate) user: User,
    pub(crate) server_experiments: Vec<ServerExperiment>,
    pub(crate) llms: crate::ai::llms::ModelsByFeature,
}
