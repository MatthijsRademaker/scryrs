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
}
