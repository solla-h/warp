pub mod ids;
mod server_timestamp;
mod uint32;

pub use ids::*;
pub use server_timestamp::ServerTimestamp;
pub use uint32::Uint32;

#[cfg(feature = "graphql")]
mod graphql_scalars {
    use cynic::impl_scalar;
    pub use warp_graphql_schema::schema;
    impl_scalar!(crate::ServerTimestamp, schema::Time);
    impl_scalar!(crate::Uint32, schema::Uint);
}
