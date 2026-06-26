//! Sharing support stubs. The drive sharing module has been removed.

use warp_core::ui::appearance::Appearance;
use warpui::elements::{Fill, MouseStateHandle, ParentElement};
use warpui::{AppContext, ViewContext};

use super::{PaneHeader};
use crate::pane_group::BackingView;
use crate::server::telemetry::SharingDialogSource;

/// Pane header component for sharing the pane contents (stubbed out).
pub struct SharedPaneContent {
    _primary_button_handle: MouseStateHandle,
}

impl SharedPaneContent {
    pub fn new<P: BackingView>(_ctx: &mut ViewContext<PaneHeader<P>>) -> Self {
        Self {
            _primary_button_handle: Default::default(),
        }
    }
}

impl<P: BackingView> PaneHeader<P> {
    pub fn set_shareable_object(
        &mut self,
        _shareable_object: Option<()>,
        _ctx: &mut ViewContext<Self>,
    ) {
    }

    pub fn has_shareable_object<C: warpui::ViewAsRef>(&self, _ctx: &C) -> bool {
        false
    }

    pub fn has_shareable_shared_session<C: warpui::ViewAsRef>(&self, _ctx: &C) -> bool {
        false
    }

    pub fn is_sharing_dialog_enabled<C: warpui::ViewAsRef>(&self, _ctx: &C) -> bool {
        false
    }

    pub fn share_pane_contents(
        &mut self,
        _source: SharingDialogSource,
        _ctx: &mut ViewContext<Self>,
    ) {
    }

    pub fn open_shared_session_qr_code(
        &mut self,
        _source: SharingDialogSource,
        _ctx: &mut ViewContext<Self>,
    ) {
    }

    pub fn render_sharing_controls(
        &self,
        _element: &mut impl ParentElement,
        _appearance: &Appearance,
        _icon_color_override: Option<Fill>,
        _button_size_override: Option<f32>,
        _app: &AppContext,
    ) {
    }
}
