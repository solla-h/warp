pub use object_models::{AIFact, AIMemory, CloudAIFact, CloudAIFactModel};
use warp_core::ui::appearance::Appearance;

use crate::objects::model::generic_string_model::StringModel;
use crate::objects::model::json_model::JsonModel;
use crate::objects::{CloudObjectTypeAndId, 
    GenericStringObjectFormat, GenericStringObjectUniqueKey, JsonObjectType, Revision,
};
use crate::ids::SyncId;

pub mod manager;
pub mod view;
pub use manager::AIFactManager;
pub use view::{AIFactView, AIFactViewEvent};

impl StringModel for AIFact {
    type CloudObjectType = CloudAIFact;

    fn model_type_name(&self) -> &'static str {
        "Rule"
    }

    fn should_enforce_revisions() -> bool {
        true
    }

    fn model_format() -> GenericStringObjectFormat {
        GenericStringObjectFormat::Json(JsonObjectType::AIFact)
    }

    fn should_show_activity_toasts() -> bool {
        true
    }

    fn warn_if_unsaved_at_quit() -> bool {
        true
    }

    fn display_name(&self) -> String {
        match self {
            AIFact::Memory(memory) => memory.content.clone(),
        }
    }
    fn uniqueness_key(&self) -> Option<GenericStringObjectUniqueKey> {
        None
    }


}

impl JsonModel for AIFact {
    fn json_object_type() -> JsonObjectType {
        JsonObjectType::AIFact
    }
}
