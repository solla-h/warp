#![allow(dead_code, unused_variables, unused_imports)]
pub mod update_manager {
    pub use crate::cloud_object::{
        FetchSingleObjectOption, InitiatedBy, ObjectOperation, ObjectOperationResult,
        OperationSuccessType, UpdateManager, UpdateManagerEvent,
    };
    pub use cloud_object_models::InitialLoadResponse;
}