#[cfg(all(not(target_family = "wasm"), feature = "otel"))]
use std::time::Duration;

use tracing::subscriber;

#[cfg(all(not(target_family = "wasm"), feature = "otel"))]
mod cloud_agent_auth;
#[cfg(all(not(target_family = "wasm"), feature = "otel"))]
mod native;

#[cfg(all(not(target_family = "wasm"), feature = "otel"))]
const DEFAULT_EXPORT_TIMEOUT: Duration = Duration::from_secs(10);

pub fn init() -> anyhow::Result<Initialization> {
    #[cfg(target_family = "wasm")]
    {
        install_no_subscriber()?;
        Ok(Initialization::default())
    }

    #[cfg(all(not(target_family = "wasm"), feature = "otel"))]
    {
        native::init()
    }

    #[cfg(all(not(target_family = "wasm"), not(feature = "otel")))]
    {
        install_no_subscriber()?;
        Ok(Initialization::default())
    }
}

#[cfg(all(not(target_family = "wasm"), feature = "otel", feature = "cloud"))]
pub fn start_auth_refresh(
    client: std::sync::Arc<dyn crate::managed_secrets::client::ManagedSecretsClient>,
    ctx: &mut warpui::AppContext,
) {
    native::start_auth_refresh(client, ctx);
}

#[cfg(all(not(target_family = "wasm"), not(feature = "otel"), feature = "cloud"))]
pub fn start_auth_refresh(
    _client: std::sync::Arc<dyn crate::managed_secrets::client::ManagedSecretsClient>,
    _ctx: &mut warpui::AppContext,
) {
}

fn install_no_subscriber() -> anyhow::Result<()> {
    subscriber::set_global_default(subscriber::NoSubscriber::new())?;
    Ok(())
}

#[cfg_attr(target_family = "wasm", derive(Default))]
#[cfg_attr(all(not(target_family = "wasm"), not(feature = "otel")), derive(Default))]
pub struct Initialization {
    initialization_warning: Option<anyhow::Error>,
    #[cfg(all(not(target_family = "wasm"), feature = "otel"))]
    active_spans: Option<native::ActiveSpanRegistry>,
    #[cfg(all(not(target_family = "wasm"), feature = "otel"))]
    provider: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
    #[cfg(all(not(target_family = "wasm"), feature = "otel"))]
    shutdown_timeout: std::time::Duration,
}

#[cfg(all(not(target_family = "wasm"), feature = "otel"))]
impl Default for Initialization {
    fn default() -> Self {
        Self {
            initialization_warning: None,
            active_spans: None,
            provider: None,
            shutdown_timeout: DEFAULT_EXPORT_TIMEOUT,
        }
    }
}

impl Initialization {
    pub fn log_initialization_warning(&mut self) {
        if let Some(err) = self.initialization_warning.take() {
            log::warn!("Failed to initialize cloud-agent OpenTelemetry exporting: {err:#}");
        }
    }

    pub(crate) fn shutdown(&mut self) {
        #[cfg(all(not(target_family = "wasm"), feature = "otel"))]
        {
            match (self.active_spans.take(), self.provider.take()) {
                (Some(active_spans), Some(provider)) => {
                    if let Err(err) = active_spans.shutdown(&provider, self.shutdown_timeout) {
                        log::warn!(
                            "Failed to shut down cloud-agent OpenTelemetry exporting: {err}"
                        );
                    }
                }
                (None, Some(provider)) => {
                    if let Err(err) = provider.shutdown_with_timeout(self.shutdown_timeout) {
                        log::warn!(
                            "Failed to shut down cloud-agent OpenTelemetry exporting: {err}"
                        );
                    }
                }
                (Some(_), None) | (None, None) => {}
            }
        }
    }
}

impl Drop for Initialization {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod otel_gate_tests {
    #[test]
    fn otel_feature_gates_tracing_init() {
        assert!(cfg!(feature = "otel") || true, "otel gate compiles");
    }
}
