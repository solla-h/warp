#![allow(dead_code, unused_variables)]
use warpui::{AppContext, Element, Entity, View, ViewContext};
use crate::appearance::Appearance;

pub struct EmptyTrashConfirmationDialog;
impl EmptyTrashConfirmationDialog {
    pub fn new() -> Self { Self }
}

#[derive(Debug, Clone)]
pub enum EmptyTrashConfirmationEvent { Confirmed, Cancelled }

impl Entity for EmptyTrashConfirmationDialog { type Event = EmptyTrashConfirmationEvent; }
impl View for EmptyTrashConfirmationDialog {
    fn ui_name() -> &'static str { "EmptyTrashConfirmationDialog" }
    fn render(&self, _ctx: &AppContext) -> Box<dyn Element> {
        Box::new(warpui::elements::Empty::new())
    }
}
