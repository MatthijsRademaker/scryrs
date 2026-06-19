//! Shared contracts for scryrs workspace crates.

/// Version for machine-facing contracts emitted by this scaffold.
pub const SCHEMA_VERSION: &str = "0.1.0";

/// Suite component metadata used by feature-gated crates and CLI output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureDescriptor {
    pub id: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
}

/// Raw agent trace event shape. Detailed ingestion comes later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceEvent {
    pub kind: TraceEventKind,
    pub subject: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceEventKind {
    FileRead,
    Search,
    SymbolInspect,
    CommandRun,
    DocRead,
    Edit,
    FailedLookup,
}

/// Ranked knowledge hotspot from deterministic analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hotspot {
    pub subject: String,
    pub score: u32,
}

/// Knowledge graph node placeholder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNode {
    pub id: String,
    pub title: String,
}

/// Reviewable knowledge proposal placeholder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeProposal {
    pub title: String,
    pub rationale: String,
}

/// Runtime routing hint placeholder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteHint {
    pub target: String,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_version_starts_at_initial_scaffold_version() {
        assert_eq!(SCHEMA_VERSION, "0.1.0");
    }
}
