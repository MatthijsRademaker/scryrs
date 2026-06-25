//! Standalone trace and hotspot foundation for scryrs.

pub mod ingestion;
pub mod query;
pub mod scoring;
pub mod store;

pub use ingestion::{
    AcceptedEvent, DetailedIngestionOutcome, IngestionOutcome, Rejection, ingest_jsonl,
    ingest_jsonl_detailed,
};
pub use query::{QueryError, TraceQuery};
pub use scoring::{base_weight, per_event_contribution, score_hotspots};
pub use store::{CANONICAL_STORE_PATH, EventStore};

use scryrs_types::FeatureDescriptor;

/// Return weight table constants for documentation and testing.
pub fn scoring_weight_table() -> Vec<(&'static str, u32)> {
    vec![
        ("FileOpened", scoring::WEIGHT_FILE_OPENED),
        ("SearchRun", scoring::WEIGHT_SEARCH_RUN),
        ("SymbolInspected", scoring::WEIGHT_SYMBOL_INSPECTED),
        ("CommandExecuted", scoring::WEIGHT_COMMAND_EXECUTED),
        ("DocRetrieved", scoring::WEIGHT_DOC_RETRIEVED),
        ("EditMade", scoring::WEIGHT_EDIT_MADE),
        ("FailedLookup", scoring::WEIGHT_FAILED_LOOKUP),
    ]
}

/// Failure bonus applied to events with Outcome::Failure.
pub const FAILURE_BONUS: u32 = scoring::FAILURE_BONUS;

pub fn descriptor() -> FeatureDescriptor {
    FeatureDescriptor {
        id: "core",
        title: "scryrs-core",
        summary: "standalone trace ingestion and hotspot detection foundation",
    }
}
