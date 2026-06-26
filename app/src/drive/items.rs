#![allow(dead_code, unused_variables, unused_imports)]
use std::sync::{Arc, Mutex};
use cloud_objects::drive::CloudObjectTypeAndId;
use warpui::AppContext;
use warpui::elements::MouseStateHandle;
use crate::cloud_object::Space;
use crate::server::ids::{ClientId, SyncId};
use crate::ui_components::menu_button::MenuDirection;

pub mod ai_fact_collection {
    use crate::server::ids::ClientId;
    #[derive(Clone)]
    pub struct WarpDriveAIFactCollection { client_id: ClientId }
    impl WarpDriveAIFactCollection {
        pub fn new(client_id: ClientId) -> Self { Self { client_id } }
        pub fn id(&self) -> &ClientId { &self.client_id }
    }
}

pub mod folder {
    use cloud_objects::drive::CloudObjectTypeAndId;
    use cloud_object_models::CloudFolder;
    use super::WarpDriveItem;
    #[derive(Clone)]
    pub struct WarpDriveFolder {
        pub cloud_object_type_and_id: CloudObjectTypeAndId,
    }
    impl WarpDriveFolder {
        pub fn new(id: CloudObjectTypeAndId, _folder: CloudFolder) -> Self {
            Self { cloud_object_type_and_id: id }
        }
    }
    impl WarpDriveItem for WarpDriveFolder {}
}

pub mod item {
    use std::sync::{Arc, Mutex};
    use warpui::AppContext;
    use warpui::elements::Element;
    use crate::appearance::Appearance;
    use crate::cloud_object::Space;
    use crate::ui_components::menu_button::MenuDirection;

    pub fn tools_panel_menu_direction(_app: &AppContext) -> MenuDirection { MenuDirection::Right }

    #[derive(Clone, Default)]
    pub struct MouseState { hovered: bool }
    impl MouseState {
        pub fn is_hovered(&self) -> bool { self.hovered }
    }

    #[derive(Clone)]
    pub struct ItemStates {
        pub item_hover_state: Arc<Mutex<MouseState>>,
    }
    impl Default for ItemStates {
        fn default() -> Self {
            Self { item_hover_state: Arc::new(Mutex::new(MouseState::default())) }
        }
    }

    pub struct WarpDriveRow;
    impl WarpDriveRow {
        pub fn new(
            _item: Box<dyn std::any::Any>,
            _item_states: ItemStates,
            _space: Space,
            _depth: usize,
            _menu: Option<crate::menu::Menu>,
            _can_move: bool,
            _has_menu_items: bool,
            _is_dragging: bool,
            _share_dialog_open: bool,
            _is_selected: bool,
            _is_focused: bool,
            _sync_queue_is_dequeueing: bool,
            _menu_direction: MenuDirection,
            _appearance: &Appearance,
        ) -> Option<Self> { Some(Self) }
        pub fn new_from_cloud_object<A: 'static, B: 'static, C: 'static, D: 'static, E: 'static, F: 'static, G: 'static, H: 'static, I: 'static, J: 'static, K: 'static, L: 'static, M: 'static, N: 'static>(_a: A, _b: B, _c: C, _d: D, _e: E, _f: F, _g: G, _h: H, _i: I, _j: J, _k: K, _l: L, _m: M, _n: N) -> Option<Self> { Some(Self) }
        pub fn build(self) -> Self { self }
        pub fn finish(self) -> Box<dyn Element> { Box::new(warpui::elements::Empty::new()) }
    }
}

pub mod mcp_server_collection {
    use crate::server::ids::ClientId;
    #[derive(Clone)]
    pub struct WarpDriveMCPServerCollection { client_id: ClientId }
    impl WarpDriveMCPServerCollection {
        pub fn new(client_id: ClientId) -> Self { Self { client_id } }
        pub fn id(&self) -> &ClientId { &self.client_id }
    }
}

pub trait WarpDriveItem: Send + Sync {
    fn renders_in_warp_drive(&self) -> bool { false }
    fn to_warp_drive_item(&self) -> Option<Box<dyn std::any::Any>> { None }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WarpDriveItemId {
    Object(CloudObjectTypeAndId),
    Folder(SyncId),
    Space(Space),
    AIFactCollection,
    MCPServerCollection,
    Trash,
}
impl WarpDriveItemId {
    pub fn drive_row_position_id(&self) -> String { String::new() }
}