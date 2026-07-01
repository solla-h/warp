#![allow(dead_code, unused_variables)]
use warpui::{AppContext, Entity, ModelContext, SingletonEntity};
use super::DriveSortOrder;

#[derive(Debug, Clone)]
pub enum WarpDriveSettingsChangedEvent { Changed, EnableWarpDrive }

pub struct DriveSortOrderSetting { value: DriveSortOrder }
impl DriveSortOrderSetting {
    pub fn new(_key: &str, default: DriveSortOrder) -> Self { Self { value: default } }
    pub fn value(&self) -> &DriveSortOrder { &self.value }
    pub fn set_value<C: 'static>(&mut self, _value: DriveSortOrder, _ctx: &mut C) -> Result<(), ()> { Ok(()) }
}

pub struct WarpDriveSettings {
    pub sorting_choice: DriveSortOrderSetting,
    pub sharing_onboarding_block_shown: BoolSetting,
}

impl Default for WarpDriveSettings {
    fn default() -> Self { Self { sorting_choice: DriveSortOrderSetting::new("warp_drive_sorting_choice", DriveSortOrder::Alphabetical), sharing_onboarding_block_shown: BoolSetting(false) } }
}

impl WarpDriveSettings {
    pub fn is_warp_drive_enabled(_ctx: &AppContext) -> bool { true }
}

impl Entity for WarpDriveSettings { type Event = WarpDriveSettingsChangedEvent; }
impl SingletonEntity for WarpDriveSettings {}



pub struct BoolSetting(pub bool);
impl BoolSetting {
    pub fn value(&self) -> &bool { &self.0 }
    pub fn set_value<C: 'static>(&mut self, value: bool, _ctx: &mut C) -> Result<(), ()> { self.0 = value; Ok(()) }
}
impl Default for BoolSetting { fn default() -> Self { Self(false) } }

