#![allow(dead_code, unused_variables)]
use warpui::{AppContext, Element, Entity, View, ViewContext, ViewHandle};
use crate::appearance::Appearance;
use crate::cloud_object::Space;
use crate::editor::EditorView;

#[derive(Debug, Clone)]
pub enum CloudObjectNamingDialogEvent { Close }

#[derive(Clone)]
pub struct CloudObjectNamingDialog {
    pub title_editor: ViewHandle<EditorView>,
}
impl CloudObjectNamingDialog {
    pub fn new(title_editor: ViewHandle<EditorView>) -> Self { Self { title_editor } }
    pub fn close<C: 'static>(&mut self, _ctx: &mut C) {}
    pub fn is_open_for_space(&self, _space: &Space) -> bool { false }
    pub fn open<A: 'static, B: 'static, C: 'static, D: 'static, E: 'static>(&mut self, _a: A, _b: B, _c: C, _d: D, _e: E) {}
    pub fn render(&self, _appearance: &Appearance, _ctx: &AppContext) -> Box<dyn Element> { Box::new(warpui::elements::Empty::new()) }
    pub fn title(&self, _ctx: &AppContext) -> String { String::new() }
}
impl Entity for CloudObjectNamingDialog { type Event = CloudObjectNamingDialogEvent; }
