//! Test infrastructure for `chat_stream` refactoring.
//!
//! This module provides:
//! - Yakbak HTTP record/replay for offline streaming tests
//! - Minimal builders for `ByopOutputInput` and `RequestParams`
//! - Custom assertion helpers for `ResponseEvent` sequences
//! - Stream collector utilities
//! - Real provider integration tests (gated by env vars)

// TODO: Enable these modules once type paths are fixed
// pub mod assertions;
// pub mod builders;
// pub mod fixtures;
// pub mod stream_collector;
// pub mod yakbak_harness;

#[cfg(test)]
mod context_tests;

// #[cfg(test)]
// pub mod real_provider_tests;
