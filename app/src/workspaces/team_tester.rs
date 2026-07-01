#![allow(dead_code, unused_variables)]
use warpui::{Entity, ModelContext, SingletonEntity};
#[derive(Debug, Clone)]
pub enum TeamTesterStatusEvent { InitiateDataPollers }
#[derive(Default)]
pub struct TeamTesterStatus;
impl TeamTesterStatus {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self { Self }
    pub fn mock(_ctx: &mut ModelContext<Self>) -> Self { Self }
    pub fn initiate_data_pollers<A: 'static>(&mut self, _a: A, _ctx: &mut ModelContext<Self>) {}
}
impl Entity for TeamTesterStatus { type Event = TeamTesterStatusEvent; }
impl SingletonEntity for TeamTesterStatus {}
