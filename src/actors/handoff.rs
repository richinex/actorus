//! Handoff Protocol for Multi-Agent Coordination
//!
//! This module provides structured handoff protocols between agents,
//! ensuring data quality and contract compliance.
//!
//! Information Hiding:
//! - Hides schema validation logic
//! - Hides handoff contract details
//! - Exposes simple validate_handoff() interface

use crate::actors::messages::{
    AgentResponse, OutputMetadata, OutputSchema, ValidationError, ValidationResult, ValidationRule,
    ValidationType,
};
use crate::actors::validation::OutputValidator;
use serde_json::Value;
use std::collections::HashMap;

/// Handoff coordinator for multi-agent systems
#[derive(Clone)]
#[allow(dead_code)]
pub struct HandoffCoordinator {
    validator: OutputValidator,
    contracts: HashMap<String, HandoffContract>,
}

/// Contract defining expected output from an agent
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct HandoffContract {
    pub from_agent: String,
    pub to_agent: Option<String>,
    pub schema: OutputSchema,
    pub max_execution_time_ms: Option<u64>,
}

impl HandoffCoordinator {
    pub fn new() -> Self {
        Self {
            validator: OutputValidator::new(),
            contracts: HashMap::new(),
        }
    }

    /// Register a handoff contract between agents
    pub fn register_contract(&mut self, name: String, contract: HandoffContract) {
        self.validator
            .register_schema(name.clone(), contract.schema.clone());
        self.contracts.insert(name, contract);
    }

    /// Validate agent output against a handoff contract
    pub fn validate_handoff(
        &self,
        contract_name: &str,
        response: &AgentResponse,
    ) -> ValidationResult {
        let contract = match self.contracts.get(contract_name) {
            Some(c) => c,
            None => {
                return ValidationResult::failure(vec![ValidationError {
                    field: "contract".to_string(),
                    error_type: "ContractNotFound".to_string(),
                    message: format!("Handoff contract '{}' not registered", contract_name),
                    expected: None,
                    actual: None,
                }]);
            }
        };

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Extract metadata and result from response
        let (result_str, metadata) = match response {
            AgentResponse::Success {
                result, metadata, ..
            } => (result, metadata.as_ref()),
            AgentResponse::Failure { .. } => {
                errors.push(ValidationError {
                    field: "response".to_string(),
                    error_type: "AgentFailure".to_string(),
                    message: "Agent failed to complete task".to_string(),
                    expected: Some("Success".to_string()),
                    actual: Some("Failure".to_string()),
                });
                return ValidationResult::failure(errors);
            }
            AgentResponse::Timeout { .. } => {
                errors.push(ValidationError {
                    field: "response".to_string(),
                    error_type: "AgentTimeout".to_string(),
                    message: "Agent timed out before completing task".to_string(),
                    expected: Some("Success".to_string()),
                    actual: Some("Timeout".to_string()),
                });
                return ValidationResult::failure(errors);
            }
        };

        // Validate metadata if present
        if let Some(meta) = metadata {
            // Check execution time limit
            if let Some(max_time) = contract.max_execution_time_ms {
                if meta.execution_time_ms > max_time {
                    warnings.push(format!(
                        "Execution time ({}ms) exceeded limit ({}ms)",
                        meta.execution_time_ms, max_time
                    ));
                }
            }

            // Validate against schema if validation result is present
            if let Some(validation) = &meta.validation_result {
                if !validation.valid {
                    for error in &validation.errors {
                        errors.push(error.clone());
                    }
                }
            }
        }

        // Try to parse result as JSON for schema validation
        match serde_json::from_str::<Value>(result_str) {
            Ok(json_value) => {
                let schema_validation = self.validator.validate(contract_name, &json_value);
                if !schema_validation.valid {
                    errors.extend(schema_validation.errors);
                }
                warnings.extend(schema_validation.warnings);
            }
            Err(_) => {
                // Result is not JSON - validate as string
                if contract.schema.field_types.values().any(|t| t != "string") {
                    warnings.push(format!(
                        "Result is not valid JSON, but schema expects structured data"
                    ));
                }
            }
        }

        if errors.is_empty() {
            ValidationResult::success().with_warnings(warnings)
        } else {
            ValidationResult::failure(errors).with_warnings(warnings)
        }
    }

    /// Create a default database query output contract
    #[allow(dead_code)]
    pub fn database_output_contract() -> HandoffContract {
        let mut field_types = HashMap::new();
        field_types.insert("data".to_string(), "array".to_string());
        field_types.insert("row_count".to_string(), "number".to_string());

        HandoffContract {
            from_agent: "database_agent".to_string(),
            to_agent: Some("analysis_agent".to_string()),
            schema: OutputSchema {
                schema_version: "1.0".to_string(),
                required_fields: vec!["data".to_string()],
                optional_fields: vec!["row_count".to_string(), "query".to_string()],
                field_types,
                validation_rules: vec![ValidationRule {
                    field: "row_count".to_string(),
                    rule_type: ValidationType::Range,
                    constraint: "0..1000000".to_string(),
                }],
            },
            max_execution_time_ms: Some(30000),
        }
    }

    /// Create a default analysis output contract
    #[allow(dead_code)]
    pub fn analysis_output_contract() -> HandoffContract {
        let mut field_types = HashMap::new();
        field_types.insert("insights".to_string(), "array".to_string());
        field_types.insert("metrics".to_string(), "object".to_string());

        HandoffContract {
            from_agent: "analysis_agent".to_string(),
            to_agent: Some("reporting_agent".to_string()),
            schema: OutputSchema {
                schema_version: "1.0".to_string(),
                required_fields: vec!["insights".to_string()],
                optional_fields: vec!["metrics".to_string(), "recommendations".to_string()],
                field_types,
                validation_rules: vec![ValidationRule {
                    field: "insights".to_string(),
                    rule_type: ValidationType::MinLength,
                    constraint: "1".to_string(),
                }],
            },
            max_execution_time_ms: Some(60000),
        }
    }
}

impl Default for HandoffCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Enrich metadata with validation results
#[allow(dead_code)]
pub fn enrich_metadata_with_validation(
    metadata: Option<OutputMetadata>,
    validation: ValidationResult,
    schema_version: String,
) -> OutputMetadata {
    let mut meta = metadata.unwrap_or_default();
    meta.validation_result = Some(validation);
    meta.schema_version = Some(schema_version);
    meta
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::messages::CompletionStatus;

    #[test]
    fn test_handoff_validation_success() {
        let mut coordinator = HandoffCoordinator::new();
        coordinator.register_contract(
            "test_contract".to_string(),
            HandoffContract {
                from_agent: "agent_a".to_string(),
                to_agent: Some("agent_b".to_string()),
                schema: OutputSchema {
                    schema_version: "1.0".to_string(),
                    required_fields: vec!["result".to_string()],
                    optional_fields: vec![],
                    field_types: HashMap::new(),
                    validation_rules: vec![],
                },
                max_execution_time_ms: Some(5000),
            },
        );

        let response = AgentResponse::Success {
            result: r#"{"result": "success"}"#.to_string(),
            steps: vec![],
            metadata: Some(OutputMetadata {
                confidence: 0.9,
                execution_time_ms: 1000,
                ..Default::default()
            }),
            completion_status: Some(CompletionStatus::Complete { confidence: 0.9 }),
        };

        let validation = coordinator.validate_handoff("test_contract", &response);
        assert!(validation.valid);
    }

    #[test]
    fn test_handoff_validation_timeout_warning() {
        let mut coordinator = HandoffCoordinator::new();
        coordinator.register_contract(
            "test_contract".to_string(),
            HandoffContract {
                from_agent: "agent_a".to_string(),
                to_agent: Some("agent_b".to_string()),
                schema: OutputSchema {
                    schema_version: "1.0".to_string(),
                    required_fields: vec![],
                    optional_fields: vec![],
                    field_types: HashMap::new(),
                    validation_rules: vec![],
                },
                max_execution_time_ms: Some(1000),
            },
        );

        let response = AgentResponse::Success {
            result: "success".to_string(),
            steps: vec![],
            metadata: Some(OutputMetadata {
                confidence: 0.9,
                execution_time_ms: 2000, // Exceeds timeout
                ..Default::default()
            }),
            completion_status: Some(CompletionStatus::Complete { confidence: 0.9 }),
        };

        let validation = coordinator.validate_handoff("test_contract", &response);
        assert!(validation.valid); // Still valid, just a warning
        assert!(!validation.warnings.is_empty());
        assert!(validation.warnings[0].contains("Execution time"));
    }
}
