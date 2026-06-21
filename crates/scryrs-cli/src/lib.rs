//! v0 CLI contract: `scryrs hotspots <PATH>`, `scryrs record --stdin|--file <PATH>`,
//! and `scryrs init --agent <NAME>`.

mod init;

#[cfg(feature = "core")]
mod chrono;
mod dispatch;
mod help_json;
mod help_text;
mod hotspots;
mod record;
#[cfg(feature = "core")]
pub(crate) mod store_override;

pub use dispatch::{run, run_with_io, run_with_writers};

pub mod test_support;

#[cfg(test)]
mod dispatch_tests;
#[cfg(all(test, feature = "core"))]
mod hotspot_integration_tests;
#[cfg(test)]
mod init_tests;
#[cfg(all(test, feature = "core"))]
mod record_tests;
#[cfg(test)]
mod smoke_tests;
