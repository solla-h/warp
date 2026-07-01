#![allow(dead_code, unused_variables)]
mod inheritance;
use warpui::{AppContext, Element, Entity, View, ViewContext};

pub fn init(_ctx: &mut AppContext) {}

pub struct SharingDialog;
impl SharingDialog {
    pub fn new<A: 'static>(_a: A, _ctx: &mut ViewContext<Self>) -> Self { Self }
}

#[derive(Debug, Clone)]
pub enum SharingDialogEvent {}

impl Entity for SharingDialog { type Event = SharingDialogEvent; }
impl View for SharingDialog {
    fn ui_name() -> &'static str { "SharingDialog" }
    fn render(&self, _ctx: &AppContext) -> Box<dyn Element> {
        Box::new(warpui::elements::Empty::new())
    }
}
