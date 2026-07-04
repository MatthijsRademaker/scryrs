#![recursion_limit = "256"]

//! v0 CLI contract: `scryrs hotspots <PATH>`, `scryrs record --stdin|--file <PATH>`,
//! `scryrs init --agent <NAME>`, `scryrs up`, and `scryrs dashboard`.

mod dashboard;
mod doctor;
mod init;
mod init_prompt;
mod live_bootstrap;
mod server;
mod setup;
mod up;

#[cfg(feature = "core")]
mod chrono;
mod dispatch;
mod graph;
mod help_json;
mod help_text;
mod hook;
mod hotspots;
mod proposals;
mod propose;
mod publish;
mod record;
#[cfg(feature = "core")]
mod remote_config;
#[cfg(feature = "core")]
mod remote_submit;
mod route;
mod route_explain;
#[cfg(feature = "core")]
pub(crate) mod store_override;

pub use dispatch::{run, run_with_io, run_with_writers};

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod dispatch_tests;
#[cfg(all(test, feature = "core"))]
mod hook_tests;
#[cfg(all(test, feature = "core"))]
mod hotspot_integration_tests;
#[cfg(test)]
mod init_tests;
#[cfg(test)]
mod proposals_tests;
#[cfg(test)]
mod publish_tests;
#[cfg(all(test, feature = "core"))]
mod record_tests;
#[cfg(test)]
mod setup_tests;
#[cfg(test)]
mod smoke_tests;
#[cfg(test)]
mod up_tests;
