#![allow(dead_code, unused_variables)]
pub mod dialog;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharingAccessLevel { View, Edit }
impl Default for SharingAccessLevel { fn default() -> Self { Self::View } }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentEditability { Editable, ReadOnly }
impl Default for ContentEditability { fn default() -> Self { Self::ReadOnly } }
impl ContentEditability {
    pub fn can_edit(&self) -> bool { matches!(self, Self::Editable) }
}
pub trait ShareableObject {}
