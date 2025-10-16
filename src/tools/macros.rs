//! Tool Definition Macros
//!
//! Simplifies tool creation by reducing boilerplate

/// Define tool metadata using a declarative syntax
///
/// # Example
/// ```
/// tool_metadata! {
///     name: "append_file",
///     description: "Append content to an existing file",
///     parameters: [
///         {
///             name: "path",
///             type: "string",
///             description: "The file path to append to",
///             required: true
///         },
///         {
///             name: "content",
///             type: "string",
///             description: "The content to append",
///             required: true
///         }
///     ]
/// }
/// ```
#[macro_export]
macro_rules! tool_metadata {
    (
        name: $name:expr,
        description: $description:expr,
        parameters: [
            $(
                {
                    name: $param_name:expr,
                    type: $param_type:expr,
                    description: $param_desc:expr,
                    required: $param_required:expr
                }
            ),* $(,)?
        ]
    ) => {
        $crate::tools::ToolMetadata {
            name: $name.to_string(),
            description: $description.to_string(),
            parameters: vec![
                $(
                    $crate::tools::ToolParameter {
                        name: $param_name.to_string(),
                        param_type: $param_type.to_string(),
                        description: $param_desc.to_string(),
                        required: $param_required,
                    }
                ),*
            ],
        }
    };
}

/// Validate required string parameter
#[macro_export]
macro_rules! validate_required_string {
    ($args:expr, $param:expr) => {
        $args[$param].as_str().ok_or_else(|| {
            anyhow::anyhow!("'{}' parameter is required and must be a string", $param)
        })?
    };
}

/// Validate optional string parameter
#[macro_export]
macro_rules! validate_optional_string {
    ($args:expr, $param:expr, $default:expr) => {
        $args[$param].as_str().unwrap_or($default)
    };
}

/// Validate required number parameter
#[macro_export]
macro_rules! validate_required_number {
    ($args:expr, $param:expr) => {
        $args[$param].as_i64().ok_or_else(|| {
            anyhow::anyhow!("'{}' parameter is required and must be a number", $param)
        })?
    };
}

/// Generate tool result helpers
#[macro_export]
macro_rules! tool_result {
    (success: $msg:expr) => {
        Ok($crate::tools::ToolResult::success($msg))
    };
    (failure: $msg:expr) => {
        Ok($crate::tools::ToolResult::failure($msg))
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_tool_metadata_macro() {
        let metadata = tool_metadata! {
            name: "test_tool",
            description: "A test tool",
            parameters: [
                {
                    name: "param1",
                    type: "string",
                    description: "First parameter",
                    required: true
                },
                {
                    name: "param2",
                    type: "number",
                    description: "Second parameter",
                    required: false
                }
            ]
        };

        assert_eq!(metadata.name, "test_tool");
        assert_eq!(metadata.description, "A test tool");
        assert_eq!(metadata.parameters.len(), 2);
        assert_eq!(metadata.parameters[0].name, "param1");
        assert_eq!(metadata.parameters[0].required, true);
        assert_eq!(metadata.parameters[1].name, "param2");
        assert_eq!(metadata.parameters[1].required, false);
    }
}
