use warp_core::ui::appearance::Appearance;
use warpui::elements::MouseStateHandle;
use warpui::ui_components::components::UiComponent;
use warpui::{AppContext, Element, Entity, SingletonEntity, TypedActionView, View, ViewContext};

use crate::server::ids::ApiKeyUid;
use crate::ui_components::buttons::icon_button;
use crate::ui_components::icons::Icon;

#[derive(PartialEq, Eq)]
enum RequestState {
    Idle,
    Pending,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExpireApiKeyButtonAction {
    ExpireApiKey,
}

pub enum ExpireApiKeyButtonEvent {
    ExpireApiKeySucceeded { uid: ApiKeyUid },
    ExpireApiKeyFailed { message: String },
}

pub struct ExpireApiKeyButton {
    key_uid: ApiKeyUid,
    button_mouse_state: MouseStateHandle,
    request_state: RequestState,
}

impl ExpireApiKeyButton {
    pub fn new(key_uid: ApiKeyUid) -> Self {
        Self {
            key_uid,
            button_mouse_state: Default::default(),
            request_state: RequestState::Idle,
        }
    }

    fn expire_api_key(&mut self, _ctx: &mut ViewContext<Self>) {
        todo!()
    }
}

impl View for ExpireApiKeyButton {
    fn ui_name() -> &'static str {
        "ExpireApiKeyButton"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let expire_icon = match self.request_state {
            RequestState::Pending => Icon::Loading,
            RequestState::Idle => Icon::Trash,
        };
        let mut expire_button = icon_button(
            appearance,
            expire_icon,
            false,
            self.button_mouse_state.clone(),
        )
        .build();
        if self.request_state != RequestState::Idle {
            expire_button = expire_button.disable();
        }
        expire_button
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(ExpireApiKeyButtonAction::ExpireApiKey);
            })
            .finish()
    }
}

impl Entity for ExpireApiKeyButton {
    type Event = ExpireApiKeyButtonEvent;
}

impl TypedActionView for ExpireApiKeyButton {
    type Action = ExpireApiKeyButtonAction;

    fn handle_action(&mut self, action: &ExpireApiKeyButtonAction, ctx: &mut ViewContext<Self>) {
        match action {
            ExpireApiKeyButtonAction::ExpireApiKey => {
                self.expire_api_key(ctx);
            }
        }
    }
}
