#![allow(dead_code, unused_variables, unused_imports)]
use std::cmp::Ordering;
use std::fmt::Debug;
pub use cloud_objects::drive::CloudObjectTypeAndId;
use cloud_objects::cloud_object::Owner;
use warpui::{AppContext, Entity, View, ViewContext};
use crate::appearance::Appearance;
use crate::cloud_object::model::view::{CloudViewModel, UpdateTimestamp};
use crate::cloud_object::CloudObject;
use crate::ids::{ServerId, SyncId};

pub mod cloud_object_naming_dialog;
pub mod cloud_object_styling;
pub mod empty_trash_confirmation_dialog;
pub mod import;
pub mod index;
pub mod items;
pub mod export;
pub mod folders;
pub mod panel;
pub mod sharing;
pub mod settings;
pub mod workflows;
pub mod drive_helpers;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveSortOrder { Alphabetical, DateModified, DateCreated, AlphabeticalAscending, AlphabeticalDescending, ByTimestamp, ByObjectType }
impl Default for DriveSortOrder { fn default() -> Self { Self::Alphabetical } }
impl DriveSortOrder {
    pub fn sort_by(
        &self,
        _cloud_view_model: &CloudViewModel,
        _timestamp: UpdateTimestamp,
        _ctx: &AppContext,
    ) -> impl FnMut(&&dyn CloudObject, &&dyn CloudObject) -> Ordering {
        |_a: &&dyn CloudObject, _b: &&dyn CloudObject| Ordering::Equal
    }
}

#[derive(Debug, Clone)]
pub struct OpenWarpDriveObjectArgs {
    pub cloud_object_type_and_id: CloudObjectTypeAndId,
    pub settings: OpenWarpDriveObjectSettings,
    pub object_type: crate::cloud_object::ObjectType,
    pub server_id: ServerId,
}

impl Default for OpenWarpDriveObjectArgs {
    fn default() -> Self {
        Self {
            cloud_object_type_and_id: CloudObjectTypeAndId::Notebook(SyncId::ClientId(crate::ids::ClientId::new())),
            settings: OpenWarpDriveObjectSettings::default(),
            object_type: crate::cloud_object::ObjectType::Notebook,
            server_id: ServerId::default(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct OpenWarpDriveObjectSettings {
    pub show_in_new_pane: bool,
    pub focused_folder_id: Option<ServerId>,
    pub invitee_email: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriveObjectType {
    Workflow, Notebook { is_ai_document: bool }, EnvVarCollection, Folder,
    AgentModeWorkflow, AIFact, AIFactCollection, MCPServer, MCPServerCollection,
}
impl Default for DriveObjectType { fn default() -> Self { Self::Workflow } }
impl std::fmt::Display for DriveObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{:?}", self) }
}

#[derive(Default)]
pub struct DrivePanel;
impl DrivePanel {
    pub fn open_cloud_object_dialog<A, B, C>(&mut self, _a: A, _b: B, _c: C, _d: &mut ViewContext<'_, Self>) {}
    pub fn reset_focused_index_in_warp_drive<A>(&mut self, _a: A, _b: &mut ViewContext<'_, Self>) {}
    pub fn scroll_item_into_view<A>(&mut self, _a: A, _b: &mut ViewContext<'_, Self>) {}
    pub fn expand_section_for_drive_item_id<A>(&mut self, _a: A, _b: &mut ViewContext<'_, Self>) {}
    pub fn initialize_drive_section_states(&mut self, _a: &mut ViewContext<'_, Self>) {}
    pub fn reset_and_open_to_main_index(&mut self, _a: &mut ViewContext<'_, Self>) {}
    pub fn set_focused_item<A>(&mut self, _a: A, _b: &mut ViewContext<'_, Self>) {}
    pub fn set_focused_index<A>(&mut self, _a: A, _b: &mut ViewContext<'_, Self>) {}
    pub fn open_object_sharing_settings<A, B, C>(&mut self, _a: A, _b: B, _c: C, _d: &mut ViewContext<'_, Self>) {}
    pub fn set_selected_object<A>(&mut self, _a: A) {}
    pub fn undo_trash<A>(&mut self, _a: A, _b: &mut ViewContext<'_, Self>) {}
    pub fn create_workflow_with_content<A, B, C, D>(&mut self, _a: A, _b: B, _c: C, _d: D, _e: &mut ViewContext<'_, Self>) {}
    pub fn move_object_to_team_owner<A, B>(&mut self, _a: A, _b: B, _c: &mut ViewContext<'_, Self>) {}
    pub fn has_warp_drive_initialized_sections(&self, _app: &AppContext) -> impl std::future::Future<Output = ()> { async {} }
}
#[derive(Clone, Debug)]
pub enum DrivePanelAction {}
impl Entity for DrivePanel { type Event = DrivePanelEvent; }
impl View for DrivePanel {
    fn ui_name() -> &'static str { "DrivePanel" }
    fn render(&self, _app: &AppContext) -> Box<dyn warpui::elements::Element> { Box::new(warpui::elements::Empty::new()) }
}
impl warpui::TypedActionView for DrivePanel {
    type Action = DrivePanelAction;
}

#[derive(Debug)]
pub enum DrivePanelEvent {
    OpenObject(CloudObjectTypeAndId),
    RunWorkflow(std::sync::Arc<crate::workflows::CloudWorkflow>),
    InvokeEnvironmentVariables { env_var_collection: std::sync::Arc<crate::env_vars::CloudEnvVarCollection>, in_subshell: bool },
    OpenTeamSettingsPage,
    OpenImportModal { owner: Owner, initial_folder_id: Option<SyncId> },
    OpenWorkflowModalWithNew { space: crate::cloud_object::Space, initial_folder_id: Option<SyncId> },
    OpenWorkflowModalWithCloudWorkflow(SyncId),
    OpenSearch,
    OpenNotebook(crate::notebooks::manager::NotebookSource),
    OpenEnvVarCollection(crate::env_vars::manager::EnvVarCollectionSource),
    OpenWorkflowInPane(crate::workflows::manager::WorkflowOpenSource, crate::workflows::WorkflowViewMode),
    OpenAIFactCollection,
    OpenMCPServerCollection,
    FocusWarpDrive,
    OpenSharedObjectsCreationDeniedModal(DriveObjectType, ServerId),
    AttachPlanAsContext(String),
}


