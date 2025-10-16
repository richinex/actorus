//! Agent Output Validation
//!
//! This module provides structured validation for agent outputs to ensure
//! quality handoffs between agents in multi-agent orchestration.
//!
//! Information Hiding:
//! - Validation logic encapsulated
//! - Schema matching rules hidden
//! - Exposes simple validate() interface

use crate::actors::messages::{
    OutputSchema, ValidationError, ValidationResult, ValidationRule, ValidationType,
};
use serde_json::Value;
use std::collections::HashMap;

/// Validator for agent outputs
#[derive(Clone)]
#[allow(dead_code)]
pub struct OutputValidator {
    schemas: HashMap<String, OutputSchema>,
}

impl OutputValidator {
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    /// Register a schema for a specific agent or output type
    pub fn register_schema(&mut self, name: String, schema: OutputSchema) {
        self.schemas.insert(name, schema);
    }

    /// Validate output against a registered schema
    pub fn validate(&self, schema_name: &str, output: &Value) -> ValidationResult {
        let schema = match self.schemas.get(schema_name) {
            Some(s) => s,
            None => {
                return ValidationResult::failure(vec![ValidationError {
                    field: "schema".to_string(),
                    error_type: "SchemaNotFound".to_string(),
                    message: format!("Schema '{}' not registered", schema_name),
                    expected: None,
                    actual: None,
                }]);
            }
        };

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate required fields
        for field in &schema.required_fields {
            if !self.has_field(output, field) {
                errors.push(ValidationError {
                    field: field.clone(),
                    error_type: "MissingRequired".to_string(),
                    message: format!("Required field '{}' is missing", field),
                    expected: Some("present".to_string()),
                    actual: Some("missing".to_string()),
                });
            }
        }

        // Validate field types
        for (field, expected_type) in &schema.field_types {
            if let Some(value) = self.get_field(output, field) {
                if !self.check_type(value, expected_type) {
                    errors.push(ValidationError {
                        field: field.clone(),
                        error_type: "TypeMismatch".to_string(),
                        message: format!(
                            "Field '{}' has wrong type. Expected: {}, Actual: {}",
                            field,
                            expected_type,
                            self.get_value_type(value)
                        ),
                        expected: Some(expected_type.clone()),
                        actual: Some(self.get_value_type(value)),
                    });
                }
            }
        }

        // Apply validation rules
        for rule in &schema.validation_rules {
            if let Some(value) = self.get_field(output, &rule.field) {
                if let Some(error) = self.apply_rule(rule, value) {
                    errors.push(error);
                }
            } else if schema.required_fields.contains(&rule.field) {
                // Already reported as missing, skip
            } else {
                warnings.push(format!(
                    "Optional field '{}' not present for validation",
                    rule.field
                ));
            }
        }

        if errors.is_empty() {
            ValidationResult::success().with_warnings(warnings)
        } else {
            ValidationResult::failure(errors).with_warnings(warnings)
        }
    }

    fn has_field(&self, output: &Value, field: &str) -> bool {
        self.get_field(output, field).is_some()
    }

    fn get_field<'a>(&self, output: &'a Value, field: &str) -> Option<&'a Value> {
        // Support dot notation for nested fields
        let parts: Vec<&str> = field.split('.').collect();
        let mut current = output;

        for part in parts {
            match current.get(part) {
                Some(v) => current = v,
                None => return None,
            }
        }

        Some(current)
    }

    fn check_type(&self, value: &Value, expected_type: &str) -> bool {
        match expected_type {
            "string" => value.is_string(),
            "number" => value.is_number(),
            "boolean" => value.is_boolean(),
            "array" => value.is_array(),
            "object" => value.is_object(),
            "null" => value.is_null(),
            _ => true, // Unknown type, allow
        }
    }

    fn get_value_type(&self, value: &Value) -> String {
        if value.is_string() {
            "string".to_string()
        } else if value.is_number() {
            "number".to_string()
        } else if value.is_boolean() {
            "boolean".to_string()
        } else if value.is_array() {
            "array".to_string()
        } else if value.is_object() {
            "object".to_string()
        } else if value.is_null() {
            "null".to_string()
        } else {
            "unknown".to_string()
        }
    }

    fn apply_rule(&self, rule: &ValidationRule, value: &Value) -> Option<ValidationError> {
        match rule.rule_type {
            ValidationType::MinLength => {
                if let Some(s) = value.as_str() {
                    if let Ok(min) = rule.constraint.parse::<usize>() {
                        if s.len() < min {
                            return Some(ValidationError {
                                field: rule.field.clone(),
                                error_type: "MinLength".to_string(),
                                message: format!(
                                    "Field '{}' is too short. Min: {}, Actual: {}",
                                    rule.field,
                                    min,
                                    s.len()
                                ),
                                expected: Some(format!("length >= {}", min)),
                                actual: Some(s.len().to_string()),
                            });
                        }
                    }
                }
            }
            ValidationType::MaxLength => {
                if let Some(s) = value.as_str() {
                    if let Ok(max) = rule.constraint.parse::<usize>() {
                        if s.len() > max {
                            return Some(ValidationError {
                                field: rule.field.clone(),
                                error_type: "MaxLength".to_string(),
                                message: format!(
                                    "Field '{}' is too long. Max: {}, Actual: {}",
                                    rule.field,
                                    max,
                                    s.len()
                                ),
                                expected: Some(format!("length <= {}", max)),
                                actual: Some(s.len().to_string()),
                            });
                        }
                    }
                }
            }
            ValidationType::Pattern => {
                if let Some(s) = value.as_str() {
                    if let Ok(re) = regex::Regex::new(&rule.constraint) {
                        if !re.is_match(s) {
                            return Some(ValidationError {
                                field: rule.field.clone(),
                                error_type: "Pattern".to_string(),
                                message: format!(
                                    "Field '{}' does not match pattern: {}",
                                    rule.field, rule.constraint
                                ),
                                expected: Some(rule.constraint.clone()),
                                actual: Some(s.to_string()),
                            });
                        }
                    }
                }
            }
            ValidationType::Range => {
                if let Some(n) = value.as_f64() {
                    // Parse range like "0..100"
                    if let Some((min_str, max_str)) = rule.constraint.split_once("..") {
                        if let (Ok(min), Ok(max)) = (min_str.parse::<f64>(), max_str.parse::<f64>())
                        {
                            if n < min || n > max {
                                return Some(ValidationError {
                                    field: rule.field.clone(),
                                    error_type: "Range".to_string(),
                                    message: format!(
                                        "Field '{}' out of range. Range: {}, Actual: {}",
                                        rule.field, rule.constraint, n
                                    ),
                                    expected: Some(rule.constraint.clone()),
                                    actual: Some(n.to_string()),
                                });
                            }
                        }
                    }
                }
            }
            ValidationType::Enum => {
                if let Some(s) = value.as_str() {
                    let allowed: Vec<&str> = rule.constraint.split(',').map(|s| s.trim()).collect();
                    if !allowed.contains(&s) {
                        return Some(ValidationError {
                            field: rule.field.clone(),
                            error_type: "Enum".to_string(),
                            message: format!(
                                "Field '{}' has invalid value. Allowed: [{}], Actual: {}",
                                rule.field, rule.constraint, s
                            ),
                            expected: Some(format!("one of: {}", rule.constraint)),
                            actual: Some(s.to_string()),
                        });
                    }
                }
            }
            ValidationType::Custom => {
                // Custom validation rules can be extended here
            }
        }

        None
    }
}

impl Default for OutputValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_required_field_validation() {
        let mut validator = OutputValidator::new();

        let schema = OutputSchema {
            schema_version: "1.0".to_string(),
            required_fields: vec!["name".to_string(), "age".to_string()],
            optional_fields: vec![],
            field_types: HashMap::new(),
            validation_rules: vec![],
        };

        validator.register_schema("person".to_string(), schema);

        // Missing required field
        let output = json!({
            "name": "Alice"
        });

        let result = validator.validate("person", &output);
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].field, "age");
    }

    #[test]
    fn test_type_validation() {
        let mut validator = OutputValidator::new();

        let mut field_types = HashMap::new();
        field_types.insert("name".to_string(), "string".to_string());
        field_types.insert("age".to_string(), "number".to_string());

        let schema = OutputSchema {
            schema_version: "1.0".to_string(),
            required_fields: vec!["name".to_string(), "age".to_string()],
            optional_fields: vec![],
            field_types,
            validation_rules: vec![],
        };

        validator.register_schema("person".to_string(), schema);

        // Wrong type
        let output = json!({
            "name": "Alice",
            "age": "thirty"
        });

        let result = validator.validate("person", &output);
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].field, "age");
        assert_eq!(result.errors[0].error_type, "TypeMismatch");
    }

    #[test]
    fn test_min_length_validation() {
        let mut validator = OutputValidator::new();

        let schema = OutputSchema {
            schema_version: "1.0".to_string(),
            required_fields: vec!["name".to_string()],
            optional_fields: vec![],
            field_types: HashMap::new(),
            validation_rules: vec![ValidationRule {
                field: "name".to_string(),
                rule_type: ValidationType::MinLength,
                constraint: "3".to_string(),
            }],
        };

        validator.register_schema("person".to_string(), schema);

        // Too short
        let output = json!({
            "name": "Al"
        });

        let result = validator.validate("person", &output);
        assert!(!result.valid);
        assert_eq!(result.errors[0].error_type, "MinLength");
    }
}
