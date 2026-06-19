//! Telemetry and privacy defaults for guardrail-safe observability.

use scryrs_types::FeatureDescriptor;

pub fn descriptor() -> FeatureDescriptor {
    FeatureDescriptor {
        id: "telemetry",
        title: "scryrs-telemetry",
        summary: "opt-in telemetry and redaction foundation",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivacyConfig {
    pub telemetry_opt_in: bool,
    pub redact_prompts: bool,
    pub redact_source: bool,
    pub redact_paths: bool,
    pub allow_remote_prompt_storage: bool,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            telemetry_opt_in: false,
            redact_prompts: true,
            redact_source: true,
            redact_paths: true,
            allow_remote_prompt_storage: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetry_is_opt_in_by_default() {
        let config = PrivacyConfig::default();

        assert!(!config.telemetry_opt_in);
        assert!(config.redact_prompts);
    }
}
