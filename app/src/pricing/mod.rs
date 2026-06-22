#[cfg(not(feature = "local-only"))]
mod full_impl;
#[cfg(not(feature = "local-only"))]
pub use full_impl::*;

#[cfg(feature = "local-only")]
mod stub_impl;
#[cfg(feature = "local-only")]
pub use stub_impl::*;
