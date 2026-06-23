pub mod auth_types;
pub mod ids;
mod server_timestamp;
mod uint32;
pub mod user_uid;

pub use auth_types::{AnonymousUserType, PersonalObjectLimits, PrincipalType, UserMetadata};
pub use ids::*;
pub use server_timestamp::ServerTimestamp;
pub use uint32::Uint32;
pub use user_uid::{TEST_USER_EMAIL, TEST_USER_UID, UserUid};

#[cfg(feature = "graphql")]
mod graphql_scalars {
    use cynic::impl_scalar;
    pub use warp_graphql_schema::schema;
    impl_scalar!(crate::ServerTimestamp, schema::Time);
    impl_scalar!(crate::Uint32, schema::Uint);
}

#[cfg(feature = "graphql")]
impl From<UserUid> for cynic::Id {
    fn from(user_uid: UserUid) -> Self {
        cynic::Id::new(user_uid.as_str())
    }
}
