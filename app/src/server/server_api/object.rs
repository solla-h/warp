use std::collections::HashMap;

use anyhow::Result;
use async_channel::Sender;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use cloud_object_models::{
    GetCloudObjectResponse, InitialLoadResponse, ObjectActionHistory, ObjectActionType,
    ObjectDeleteResult, ObjectMetadataUpdateResult, ObjectPermissionUpdateResult,
    ObjectPermissionsUpdateData, ObjectUpdateMessage,
};
pub use cloud_object_models::{GuestIdentifier, ObjectClient};
use cloud_object_models::JsonSerializer;
use warp_core::report_error;

use crate::ai::ambient_agents::scheduled::ScheduledAmbientAgent;
use crate::ai::cloud_environments::AmbientAgentEnvironment;
use crate::ai::document::ai_document_model::AIDocumentId;
use crate::ai::execution_profiles::AIExecutionProfile;
use crate::ai::facts::AIFact;
use crate::ai::mcp::{MCPServer, TemplatableMCPServer};
use crate::channel::ChannelState;
use crate::cloud_object::{
    BulkCreateCloudObjectResult, BulkCreateGenericStringObjectsRequest, CreateCloudObjectResult,
    CreateObjectRequest, GenericStringObjectFormat, GenericStringObjectUniqueKey,
    ObjectsToUpdate, ObjectType, Owner, Revision, ServerMetadata,
    ServerObject, ServerPermissions, UpdateCloudObjectResult,
};
use crate::cloud_object::{
    ServerFolder, ServerNotebook, ServerWorkflow,
};
use crate::cloud_object::model::generic_string_model::{
    GenericStringModel, GenericStringObjectId, Serializer, StringModel,
};
use cloud_objects::drive::sharing::SharingAccessLevel;
use cloud_objects::ids::{FolderId, ServerId};
use cloud_objects::cloud_object::SerializedModel;
use cloud_object_models::WorkflowId;

use super::ServerApi;

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl ObjectClient for ServerApi {
    async fn create_workflow(
        &self,
        _request: CreateObjectRequest,
    ) -> Result<CreateCloudObjectResult> {
        todo!("GraphQL backend removed")
    }

    async fn update_workflow(
        &self,
        _workflow_id: WorkflowId,
        _data: SerializedModel,
        _revision: Option<Revision>,
    ) -> Result<UpdateCloudObjectResult<ServerWorkflow>> {
        todo!("GraphQL backend removed")
    }

    async fn bulk_create_generic_string_objects(
        &self,
        _owner: Owner,
        _objects: &[BulkCreateGenericStringObjectsRequest],
    ) -> Result<BulkCreateCloudObjectResult> {
        todo!("GraphQL backend removed")
    }

    async fn create_generic_string_object(
        &self,
        _format: GenericStringObjectFormat,
        _uniqueness_key: Option<GenericStringObjectUniqueKey>,
        _request: CreateObjectRequest,
    ) -> Result<CreateCloudObjectResult> {
        todo!("GraphQL backend removed")
    }

    async fn create_notebook(
        &self,
        _request: CreateObjectRequest,
    ) -> Result<CreateCloudObjectResult> {
        todo!("GraphQL backend removed")
    }

    async fn update_notebook(
        &self,
        _notebook_id: cloud_object_models::NotebookId,
        _title: Option<String>,
        _data: Option<SerializedModel>,
        _revision: Option<Revision>,
    ) -> Result<UpdateCloudObjectResult<ServerNotebook>> {
        todo!("GraphQL backend removed")
    }

    async fn create_folder(&self, _request: CreateObjectRequest) -> Result<CreateCloudObjectResult> {
        todo!("GraphQL backend removed")
    }

    async fn update_folder(
        &self,
        _folder_id: FolderId,
        _name: SerializedModel,
    ) -> Result<UpdateCloudObjectResult<ServerFolder>> {
        todo!("GraphQL backend removed")
    }

    async fn update_generic_string_object(
        &self,
        _object_id: GenericStringObjectId,
        _model: SerializedModel,
        _revision: Option<Revision>,
    ) -> Result<UpdateCloudObjectResult<Box<dyn ServerObject>>> {
        todo!("GraphQL backend removed")
    }

    async fn grab_notebook_edit_access(
        &self,
        _notebook_id: cloud_object_models::NotebookId,
    ) -> Result<ServerMetadata> {
        todo!("GraphQL backend removed")
    }

    async fn give_up_notebook_edit_access(
        &self,
        _notebook_id: cloud_object_models::NotebookId,
    ) -> Result<ServerMetadata> {
        todo!("GraphQL backend removed")
    }

    async fn get_warp_drive_updates(
        &self,
        _message_sender: Sender<ObjectUpdateMessage>,
        _stream_ready_sender: Sender<()>,
    ) -> Result<()> {
        todo!("GraphQL backend removed")
    }

    async fn fetch_changed_objects(
        &self,
        _objects_to_update: ObjectsToUpdate,
        _force_refresh: bool,
    ) -> Result<InitialLoadResponse> {
        todo!("GraphQL backend removed")
    }

    async fn fetch_single_cloud_object(&self, _id: ServerId) -> Result<GetCloudObjectResponse> {
        todo!("GraphQL backend removed")
    }

    async fn transfer_notebook_owner(
        &self,
        _notebook_id: cloud_object_models::NotebookId,
        _owner: Owner,
    ) -> Result<bool> {
        todo!("GraphQL backend removed")
    }

    async fn transfer_workflow_owner(&self, _workflow_id: WorkflowId, _owner: Owner) -> Result<bool> {
        todo!("GraphQL backend removed")
    }

    async fn transfer_generic_string_object_owner(
        &self,
        _object_id: GenericStringObjectId,
        _owner: Owner,
    ) -> Result<bool> {
        todo!("GraphQL backend removed")
    }

    async fn trash_object(&self, _id: ServerId) -> Result<bool> {
        todo!("GraphQL backend removed")
    }

    async fn untrash_object(&self, _id: ServerId) -> Result<ObjectMetadataUpdateResult> {
        todo!("GraphQL backend removed")
    }

    async fn delete_object(&self, _id: ServerId) -> Result<ObjectDeleteResult> {
        todo!("GraphQL backend removed")
    }

    async fn empty_trash(&self, _owner: Owner) -> Result<ObjectDeleteResult> {
        todo!("GraphQL backend removed")
    }

    async fn move_object(
        &self,
        _id: ServerId,
        _folder_id: Option<FolderId>,
        _owner: Owner,
        _object_type: ObjectType,
    ) -> Result<bool> {
        todo!("GraphQL backend removed")
    }

    async fn record_object_action(
        &self,
        _id: ServerId,
        _action_type: ObjectActionType,
        _timestamp: DateTime<Utc>,
        _data: Option<String>,
    ) -> Result<ObjectActionHistory> {
        todo!("GraphQL backend removed")
    }

    async fn leave_object(&self, _id: ServerId) -> Result<ObjectDeleteResult> {
        todo!("GraphQL backend removed")
    }

    async fn set_object_link_permissions(
        &self,
        _object_id: ServerId,
        _access_level: SharingAccessLevel,
    ) -> Result<ObjectPermissionUpdateResult> {
        todo!("GraphQL backend removed")
    }

    async fn remove_object_link_permissions(
        &self,
        _object_id: ServerId,
    ) -> Result<ObjectPermissionUpdateResult> {
        todo!("GraphQL backend removed")
    }

    async fn add_object_guests(
        &self,
        _object_id: ServerId,
        _guest_emails: Vec<String>,
        _access_level: SharingAccessLevel,
    ) -> Result<ObjectPermissionsUpdateData> {
        todo!("GraphQL backend removed")
    }

    async fn update_object_guests(
        &self,
        _object_id: ServerId,
        _guest_emails: Vec<String>,
        _access_level: SharingAccessLevel,
    ) -> Result<ServerPermissions> {
        todo!("GraphQL backend removed")
    }

    async fn remove_object_guest(
        &self,
        _object_id: ServerId,
        _guest: GuestIdentifier,
    ) -> Result<ServerPermissions> {
        todo!("GraphQL backend removed")
    }

    async fn fetch_environment_last_task_run_timestamps(
        &self,
    ) -> Result<HashMap<String, DateTime<Utc>>> {
        todo!("GraphQL backend removed")
    }
}
