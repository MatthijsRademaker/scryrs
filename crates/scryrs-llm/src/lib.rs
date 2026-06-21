//! Provider-neutral LLM boundary. Models explain or draft; policy stays deterministic.

use scryrs_types::FeatureDescriptor;

pub fn descriptor() -> FeatureDescriptor {
    FeatureDescriptor {
        id: "llm",
        title: "scryrs-llm",
        summary: "bounded provider-neutral LLM transport foundation",
    }
}

pub trait ModelClient {
    fn generate(&self, request: ModelRequest) -> Result<ModelResponse, ModelError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRequest {
    pub model_id: String,
    pub mode: ModelMode,
    pub input: String,
    pub max_input_chars: usize,
    pub max_output_tokens: u32,
    pub timeout_ms: u64,
    pub allow_tools: bool,
    pub trace_id: String,
}

impl ModelRequest {
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.model_id.trim().is_empty() {
            return Err(ModelError::new("model_id must be exact and non-empty"));
        }

        if self.input.chars().count() > self.max_input_chars {
            return Err(ModelError::new("input exceeds max_input_chars"));
        }

        if self.max_output_tokens == 0 {
            return Err(ModelError::new(
                "max_output_tokens must be greater than zero",
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelMode {
    Explain,
    Suggest,
    PatchPlan,
    Eval,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelResponse {
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelError {
    pub message: String,
}

impl ModelError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_request() -> ModelRequest {
        ModelRequest {
            model_id: "exact-model-id".to_string(),
            mode: ModelMode::Explain,
            input: "test input".to_string(),
            max_input_chars: 100,
            max_output_tokens: 100,
            timeout_ms: 30_000,
            allow_tools: false,
            trace_id: "trace-1".to_string(),
        }
    }

    #[test]
    fn valid_request_passes_validation() {
        assert_eq!(valid_request().validate(), Ok(()));
    }

    #[test]
    fn empty_model_id_fails_validation() {
        let mut req = valid_request();
        req.model_id = "   ".to_string();
        assert!(req.validate().is_err());
    }

    #[test]
    fn zero_max_output_tokens_fails_validation() {
        let mut req = valid_request();
        req.max_output_tokens = 0;
        assert!(req.validate().is_err());
    }

    #[test]
    fn input_at_ceiling_passes_validation() {
        let mut req = valid_request();
        req.max_input_chars = 10;
        req.input = "1234567890".to_string();
        assert_eq!(req.validate(), Ok(()));
    }

    #[test]
    fn request_validation_enforces_input_ceiling() {
        let request = ModelRequest {
            model_id: "exact-model-id".to_string(),
            mode: ModelMode::Explain,
            input: "abcd".to_string(),
            max_input_chars: 3,
            max_output_tokens: 100,
            timeout_ms: 30_000,
            allow_tools: false,
            trace_id: "trace-1".to_string(),
        };

        assert_eq!(
            request.validate(),
            Err(ModelError::new("input exceeds max_input_chars"))
        );
    }
}
