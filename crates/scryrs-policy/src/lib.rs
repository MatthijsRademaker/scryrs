//! Deterministic policy foundation for guardrail decisions.

use scryrs_types::FeatureDescriptor;

pub fn descriptor() -> FeatureDescriptor {
    FeatureDescriptor {
        id: "policy",
        title: "scryrs-policy",
        summary: "deterministic guardrail policy foundation",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDecision {
    pub allowed: bool,
    pub reasons: Vec<String>,
}

impl PolicyDecision {
    pub fn allow() -> Self {
        Self {
            allowed: true,
            reasons: Vec::new(),
        }
    }

    pub fn deny(reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reasons: vec![reason.into()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_has_no_reasons() {
        let decision = PolicyDecision::allow();
        assert!(decision.allowed);
        assert!(decision.reasons.is_empty());
    }

    #[test]
    fn denial_carries_reason() {
        let decision = PolicyDecision::deny("write access disabled");

        assert!(!decision.allowed);
        assert_eq!(decision.reasons, vec!["write access disabled"]);
    }

    #[test]
    fn deny_preserves_exact_reason() {
        let decision = PolicyDecision::deny("rate limit exceeded");
        assert_eq!(decision.reasons, vec!["rate limit exceeded"]);
    }

    #[test]
    fn allow_then_deny_are_distinct() {
        let allowed = PolicyDecision::allow();
        let denied = PolicyDecision::deny("nope");
        assert_ne!(allowed, denied);
    }
}
