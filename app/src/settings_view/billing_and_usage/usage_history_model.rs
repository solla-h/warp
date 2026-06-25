use std::sync::Arc;

use warpui::{Entity, ModelContext, SingletonEntity};

use crate::server::server_api::ai::{AIClient, ConversationUsage};
use crate::server::server_api::ServerApiProvider;

pub struct UsageHistoryModel {
    ai_client: Arc<dyn AIClient>,
    is_loading: bool,
    has_more_entries: bool,
    entries: Vec<ConversationUsage>,
}

impl Entity for UsageHistoryModel {
    type Event = ();
}

impl SingletonEntity for UsageHistoryModel {}

impl UsageHistoryModel {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        let ai_client = ServerApiProvider::as_ref(ctx).get_ai_client();
        Self {
            ai_client,
            is_loading: false,
            has_more_entries: true,
            entries: Vec::new(),
        }
    }

    pub fn is_loading(&self) -> bool {
        self.is_loading
    }

    pub fn has_more_entries(&self) -> bool {
        self.has_more_entries
    }

    pub fn entries(&self) -> &[ConversationUsage] {
        &self.entries
    }

    pub fn refresh_usage_history_async(&mut self, _ctx: &mut ModelContext<Self>) {
        // No-op: GraphQL backend removed
    }

    pub fn load_more_usage_history_async(&mut self, _ctx: &mut ModelContext<Self>) {
        // No-op: GraphQL backend removed
    }
}
