use crate::search::mixer::SearchMixer;
use crate::ids::SyncId;

pub type EmbeddingSearchMixer = SearchMixer<EmbeddingSearchItemAction>;

#[derive(Clone, Debug)]
pub enum EmbeddingSearchItemAction {
    AcceptWorkflow(SyncId),
    AcceptNotebook(SyncId),
}
