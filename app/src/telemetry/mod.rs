#![cfg_attr(feature = "local-only", allow(dead_code, unused_imports, unused_variables))]
mod collector;
mod context;
pub mod context_provider;
mod events;
mod macros;
pub mod rudder_message;
pub mod secret_redaction;

use std::path::{Path, PathBuf};

use anyhow::Result;
pub use collector::*;
pub use context::telemetry_context;
pub use events::*;
use rudder_message::BatchMessage as RudderBatchMessageWithMetadata;
use warpui::telemetry::Event;

use crate::auth::UserUid;
use crate::settings::PrivacySettingsSnapshot;

const RUDDER_TELEMETRY_EVENTS_FILE_NAME: &str = "rudder_telemetry_events.json";

fn rudder_event_file_path() -> PathBuf {
    warp_core::paths::secure_state_dir()
        .unwrap_or_else(warp_core::paths::state_dir)
        .join(RUDDER_TELEMETRY_EVENTS_FILE_NAME)
}

pub fn clear_event_queue() {
    let _ = warpui::telemetry::flush_events();
}

pub struct TelemetryApi {
    pub(crate) client: http_client::Client,
}

impl Default for TelemetryApi {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryApi {
    pub fn new() -> Self {
        let client = http_client::Client::default();
        Self { client }
    }

    pub async fn flush_events(&self, _settings_snapshot: PrivacySettingsSnapshot) -> Result<usize> {
        let events = warpui::telemetry::flush_events();
        let event_count = events.len();
        Ok(event_count)
    }

    pub async fn flush_persisted_events_to_rudder(
        &self,
        _path: &Path,
        _settings_snapshot: PrivacySettingsSnapshot,
    ) -> Result<()> {
        Ok(())
    }

    pub fn flush_and_persist_events(
        &self,
        _max_event_count: usize,
        _settings_snapshot: PrivacySettingsSnapshot,
    ) -> Result<()> {
        Ok(())
    }

    pub async fn send_telemetry_event(
        &self,
        _user_id: Option<UserUid>,
        _anonymous_id: String,
        _event: impl warp_core::telemetry::TelemetryEvent,
        _settings_snapshot: PrivacySettingsSnapshot,
    ) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;