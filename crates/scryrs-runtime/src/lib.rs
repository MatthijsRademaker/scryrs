//! Agent-side routing and retrieval helper foundation.

use scryrs_types::{FeatureDescriptor, RouteHint};

pub fn descriptor() -> FeatureDescriptor {
    FeatureDescriptor {
        id: "runtime",
        title: "scryrs-runtime",
        summary: "agent-side routing and retrieval helper foundation",
    }
}

pub fn explain_route(target: impl Into<String>, reason: impl Into<String>) -> RouteHint {
    RouteHint {
        target: target.into(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_hint_preserves_target() {
        let hint = explain_route("docs/routing.md", "matches prompt");

        assert_eq!(hint.target, "docs/routing.md");
    }

    #[test]
    fn route_hint_preserves_reason() {
        let hint = explain_route("docs/routing.md", "matches prompt");
        assert_eq!(hint.reason, "matches prompt");
    }

    #[test]
    fn explain_route_accepts_string_types() {
        let hint = explain_route("target.md".to_string(), "because".to_string());
        assert_eq!(hint.target, "target.md");
        assert_eq!(hint.reason, "because");
    }

    #[test]
    fn different_targets_produce_different_hints() {
        let a = explain_route("a.md", "reason");
        let b = explain_route("b.md", "reason");
        assert_ne!(a, b);
    }
}
