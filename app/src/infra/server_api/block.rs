use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;

use crate::ai::generate_block_title::api::{GenerateBlockTitleRequest, GenerateBlockTitleResponse};
use crate::server::block::{Block, DisplaySetting};

#[cfg_attr(test, automock)]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait BlockClient: 'static + Send + Sync {
    /// Unshares a block identified at `block_id`.
    async fn unshare_block(&self, block_id: String) -> Result<(), anyhow::Error>;

    /// Uploads a given block to the server via the /share_block endpoint.
    async fn save_block(
        &self,
        block: &Block,
        title: Option<String>,
        show_prompt: bool,
        display_setting: DisplaySetting,
    ) -> Result<String, anyhow::Error>;

    async fn blocks_owned_by_user(&self) -> Result<Vec<Block>, anyhow::Error>;

    async fn generate_shared_block_title(
        &self,
        request: GenerateBlockTitleRequest,
    ) -> Result<GenerateBlockTitleResponse, anyhow::Error>;
}

use super::ServerApi;

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl BlockClient for ServerApi {
    async fn unshare_block(&self, _block_id: String) -> Result<(), anyhow::Error> {
        todo!("GraphQL backend removed")
    }

    async fn save_block(
        &self,
        _block: &Block,
        _title: Option<String>,
        _show_prompt: bool,
        _display_setting: DisplaySetting,
    ) -> Result<String, anyhow::Error> {
        todo!("GraphQL backend removed")
    }

    async fn blocks_owned_by_user(&self) -> Result<Vec<Block>, anyhow::Error> {
        todo!("GraphQL backend removed")
    }

    async fn generate_shared_block_title(
        &self,
        _request: GenerateBlockTitleRequest,
    ) -> Result<GenerateBlockTitleResponse, anyhow::Error> {
        todo!("GraphQL backend removed")
    }
}

