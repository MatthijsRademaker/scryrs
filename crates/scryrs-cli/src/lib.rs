//! v0 CLI contract: `scryrs hotspots <PATH>`, `scryrs record --stdin|--file <PATH>`,
//! `scryrs init --agent <NAME>`, and `scryrs dashboard`.

mod dashboard;
mod init;
mod server;

#[cfg(feature = "core")]
mod chrono;
mod dispatch;
mod graph;
mod help_json;
mod help_text;
mod hook;
mod hotspots;
mod record;
#[cfg(feature = "core")]
mod remote_config;
#[cfg(feature = "core")]
mod remote_submit;
#[cfg(feature = "core")]
pub(crate) mod store_override;

pub use dispatch::{run, run_with_io, run_with_writers};

pub mod test_support;

#[cfg(test)]
mod dispatch_tests;
#[cfg(all(test, feature = "core"))]
mod hook_tests;
#[cfg(all(test, feature = "core"))]
mod hotspot_integration_tests;
#[cfg(test)]
mod init_tests;
#[cfg(all(test, feature = "core"))]
mod record_tests;
#[cfg(test)]
mod smoke_tests;
