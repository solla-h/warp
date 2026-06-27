#![allow(dead_code, unused_variables)]
use warpui::{Entity, ModelContext, SingletonEntity};
use crate::server::ids::ServerId;
#[derive(Debug, Clone)]
pub enum TeamUpdateManagerEvent { LeaveError, LeaveSuccess, RenameTeamError, RenameTeamSuccess }
#[derive(Default)]
pub struct TeamUpdateManager;
impl TeamUpdateManager {
    pub fn new<A: 'static, B: 'static>(_a: A, _b: B, _ctx: &mut ModelContext<Self>) -> Self { Self }
    pub fn mock(_ctx: &mut ModelContext<Self>) -> Self { Self }
    pub fn refresh_workspace_metadata(&mut self, _ctx: &mut ModelContext<Self>) -> std::future::Ready<()> { std::future::ready(()) }
    pub fn stop_polling_for_workspace_metadata_updates(&mut self) {}
    pub fn leave_team<A: 'static, B: 'static>(&mut self, _a: A, _b: B, _ctx: &mut ModelContext<Self>) {}
    pub fn create_team<A: 'static, B: 'static, C: 'static>(&mut self, _a: A, _b: B, _c: C, _ctx: &mut ModelContext<Self>) {}
    pub fn rename_team<A: 'static>(&mut self, _a: A, _ctx: &mut ModelContext<Self>) {}
    pub fn set_current_workspace_uid(&mut self, _uid: ServerId, _ctx: &mut ModelContext<Self>) {}
}
impl Entity for TeamUpdateManager { type Event = TeamUpdateManagerEvent; }
impl SingletonEntity for TeamUpdateManager {}

