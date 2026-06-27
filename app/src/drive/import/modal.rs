#![allow(dead_code, unused_variables)]
use warpui::{AppContext, Entity, View, ViewContext, TypedActionView};
use warpui::elements::Element;
use crate::server::ids::HashedSqliteId;

#[derive(Debug, Clone)]
pub enum ImportModalEvent {
    OpenTargetWithHashedId(HashedSqliteId),
    Close,
}

#[derive(Debug, Clone)]
pub enum ImportModalAction {}

pub struct ImportModal;

impl ImportModal {
    pub fn new(_ctx: &mut ViewContext<Self>) -> Self { Self }
    pub fn open_with_target<A, B>(&mut self, _a: A, _b: B, _c: &mut ViewContext<'_, Self>) {}
}

impl Entity for ImportModal {
    type Event = ImportModalEvent;
}

impl View for ImportModal {
    fn ui_name() -> &'static str { "ImportModal" }
    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        Box::new(warpui::elements::Empty::new())
    }
}

impl TypedActionView for ImportModal {
    type Action = ImportModalAction;
}

