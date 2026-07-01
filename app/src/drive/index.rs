#![allow(dead_code, unused_variables, unused_imports)]
use warpui::{AppContext, Element, Entity, View, ViewContext, TypedActionView, ViewHandle};

use super::items::WarpDriveItemId;
use super::{CloudObjectTypeAndId, DriveObjectType, DriveSortOrder};
use crate::ids::{ServerId, SyncId};

pub fn init(_ctx: &mut AppContext) {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriveIndexSection {
    Personal,
    Team(ServerId),
}

#[derive(Clone, Debug)]
pub enum DriveIndexAction {
    FocusItem(WarpDriveItemId),
    SelectItem(WarpDriveItemId),
    OpenItem(WarpDriveItemId),
}

pub enum DriveIndexEvent {}

pub struct DriveIndex;
impl Entity for DriveIndex { type Event = DriveIndexEvent; }
impl View for DriveIndex {
    fn ui_name() -> &'static str { "DriveIndex" }
    fn render(&self, _app: &AppContext) -> Box<dyn warpui::elements::Element> {
        Box::new(warpui::elements::Empty::new())
    }
}
impl TypedActionView for DriveIndex {
    type Action = DriveIndexAction;
}
