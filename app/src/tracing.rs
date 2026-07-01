use tracing::subscriber;

pub fn init() -> anyhow::Result<Initialization> {
    #[cfg(target_family = "wasm")]
    {
        install_no_subscriber()?;
        Ok(Initialization::default())
    }

    #[cfg(not(target_family = "wasm"))]
    {
        install_no_subscriber()?;
        Ok(Initialization::default())
    }
}

pub fn start_auth_refresh(
    _client: std::sync::Arc<dyn crate::managed_secrets::client::ManagedSecretsClient>,
    _ctx: &mut warpui::AppContext,
) {
}

fn install_no_subscriber() -> anyhow::Result<()> {
    subscriber::set_global_default(subscriber::NoSubscriber::new())?;
    Ok(())
}

#[derive(Default)]
pub struct Initialization {
    initialization_warning: Option<anyhow::Error>,
}

impl Initialization {
    pub fn log_initialization_warning(&mut self) {
        if let Some(err) = self.initialization_warning.take() {
            log::warn!("Failed to initialize tracing: {err:#}");
        }
    }

    pub(crate) fn shutdown(&mut self) {}
}

impl Drop for Initialization {
    fn drop(&mut self) {
        self.shutdown();
    }
}
