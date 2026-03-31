use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub suggestion: Option<String>,
    pub details: serde_json::Value,
}

impl EngineError {
    pub fn invalid_request(message: String) -> Self {
        Self {
            code: "INVALID_REQUEST".to_string(),
            message,
            retryable: false,
            suggestion: None,
            details: serde_json::json!({}),
        }
    }

    pub fn unsupported_capability(capability: &str) -> Self {
        Self {
            code: "UNSUPPORTED_CAPABILITY".to_string(),
            message: format!("Capability '{}' is not implemented yet.", capability),
            retryable: false,
            suggestion: None,
            details: serde_json::json!({ "capability": capability }),
        }
    }

    pub fn backend_execution_failed(message: String) -> Self {
        Self {
            code: "BACKEND_EXECUTION_FAILED".to_string(),
            message,
            retryable: false,
            suggestion: None,
            details: serde_json::json!({}),
        }
    }

    pub fn file_not_found(path: &str) -> Self {
        Self {
            code: "FILE_NOT_FOUND".to_string(),
            message: format!(
                "Path '{}' was not found inside the configured workspace root.",
                path
            ),
            retryable: false,
            suggestion: Some(
                "Provide an existing file or directory path inside the workspace root."
                    .to_string(),
            ),
            details: serde_json::json!({ "path": path }),
        }
    }

    pub fn path_outside_workspace(path: &str) -> Self {
        Self {
            code: "PATH_OUTSIDE_WORKSPACE".to_string(),
            message: format!(
                "Path '{}' is outside the configured workspace root.",
                path
            ),
            retryable: false,
            suggestion: Some(
                "Use a path inside the workspace root or omit the path filter.".to_string(),
            ),
            details: serde_json::json!({ "path": path }),
        }
    }
}
