use std::sync::OnceLock;

use serde::Serialize;
use serde_json::{json, Value};
#[cfg(target_family = "wasm")]
use warpui::platform::wasm;

use warp_core::operating_system_info::OperatingSystemInfo;

static TELEMETRY_CONTEXT: OnceLock<TelemetryContext> = OnceLock::new();

#[derive(Serialize)]
struct TelemetryContextInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    os: Option<&'static OperatingSystemInfo>,
    #[serde(rename = "userAgent", skip_serializing_if = "Option::is_none")]
    user_agent: Option<String>,
}

pub struct TelemetryContext(Value);

impl TelemetryContext {
    pub fn as_value(&self) -> Value {
        self.0.clone()
    }
}

impl TelemetryContext {
    fn new() -> Self {
        let context = TelemetryContextInfo {
            os: OperatingSystemInfo::get().ok(),
            user_agent: user_agent(),
        };

        match serde_json::to_value(context) {
            Ok(value) => Self(value),
            Err(e) => {
                log::error!("Failed to serialize telemetry context info to JSON value: {e:?}");
                Self(json!({}))
            }
        }
    }
}

fn user_agent() -> Option<String> {
    cfg_if::cfg_if! {
        if #[cfg(target_family = "wasm")] {
            wasm::user_agent()
        } else {
            None
        }
    }
}

pub fn telemetry_context() -> &'static TelemetryContext {
    TELEMETRY_CONTEXT.get_or_init(TelemetryContext::new)
}
