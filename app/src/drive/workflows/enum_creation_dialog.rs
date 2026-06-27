#![allow(dead_code, unused_variables, unused_imports)]
use warpui::{ViewContext, View, Element, AppContext};
use crate::ids::SyncId;

#[derive(Default, Clone, Debug)]
pub struct WorkflowEnumData {
    pub id: Option<SyncId>,
    pub name: String,
    pub values: Vec<String>,
}

pub struct EnumCreationDialog;

impl EnumCreationDialog {
    pub fn new(_ctx: &mut ViewContext<Self>) -> Self { Self }
}

pub enum EnumCreationDialogEvent {
    Created(WorkflowEnumData),
    Dismissed,
}
