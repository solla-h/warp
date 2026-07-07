//! Test infrastructure for `chat_stream` refactoring.

pub mod assertions;
pub mod builders;
pub mod stream_collector;
pub mod yakbak_harness;

#[cfg(test)]
mod context_tests;
#[cfg(test)]
mod client_tests;
#[cfg(test)]
mod diagnostics_tests;
#[cfg(test)]
pub mod real_provider_tests;
