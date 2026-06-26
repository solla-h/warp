#![allow(dead_code, unused_variables, unused_imports)]
use std::sync::Arc;
use warpui::{AppContext, Element, Entity, View, ViewContext, TypedActionView};

use crate::auth::UserUid;
use crate::cloud_object::Owner;
use crate::drive::items::WarpDriveItemId;
use crate::server::ids::{ServerId, SyncId};
use crate::server::server_api::ai::AIClient;

pub struct WorkflowModal {
    is_open: bool,
}

#[derive(Clone, Debug)]
pub enum WorkflowModalAction {
    Close,
    Save,
}

pub enum WorkflowModalEvent {
    Close,
    UpdatedWorkflow(SyncId),
    AiAssistError(String),
    AiAssistUpgradeError(Option<ServerId>, UserUid),
    ViewInWarpDrive(WarpDriveItemId),
}

impl WorkflowModal {
    pub fn new(_ai_client: Arc<dyn AIClient>, _ctx: &mut ViewContext<Self>) -> Self {
        Self { is_open: false }
    }

    pub fn open_with_new(&mut self, _owner: Owner, _initial_folder_id: Option<SyncId>, _ctx: &mut ViewContext<Self>) {
        self.is_open = true;
    }

    pub fn open_with_cloud_workflow(&mut self, _workflow_id: SyncId, _ctx: &mut ViewContext<Self>) {
        self.is_open = true;
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }
}

impl Entity for WorkflowModal {
    type Event = WorkflowModalEvent;
}

impl View for WorkflowModal {
    fn ui_name() -> &'static str { "WorkflowModal" }
    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        Box::new(warpui::elements::Empty::new())
    }
}

impl TypedActionView for WorkflowModal {
    type Action = WorkflowModalAction;
}
