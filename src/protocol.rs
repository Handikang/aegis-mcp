use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    /// Extracts tool name and arguments if this request is an MCP `tools/call` method.
    pub fn extract_tool_call(&self) -> Option<(&str, Option<&Value>)> {
        if self.method != "tools/call" {
            return None;
        }

        let params = self.params.as_ref()?;
        let name = params.get("name")?.as_str()?;
        let arguments = params.get("arguments");

        Some((name, arguments))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorObject {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub jsonrpc: String,
    pub id: Value,
    pub error: ErrorObject,
}

impl JsonRpcError {
    pub fn permission_denied(id: Value, tool_name: &str, reason: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            error: ErrorObject {
                code: -32001,
                message: format!(
                    "[Aegis-MCP Security Alert] Tool execution for '{}' was BLOCKED by policy. Reason: {}. You must not attempt this dangerous action again; report this restriction and proceed with safe alternative steps.",
                    tool_name, reason
                ),
                data: None,
            },
        }
    }

    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        let mut json = serde_json::to_string(self)?;
        json.push('\n');
        Ok(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_tool_call_valid() {
        let req_json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "bash_execute",
                "arguments": {
                    "command": "cat ~/.ssh/id_rsa"
                }
            }
        }"#;

        let req: JsonRpcRequest = serde_json::from_str(req_json).unwrap();
        let (tool_name, args) = req.extract_tool_call().unwrap();

        assert_eq!(tool_name, "bash_execute");
        let cmd = args.unwrap().get("command").unwrap().as_str().unwrap();
        assert_eq!(cmd, "cat ~/.ssh/id_rsa");
    }

    #[test]
    fn test_extract_tool_call_other_method() {
        let req_json = r#"{
            "jsonrpc": "2.0",
            "id": 2,
            "method": "initialize",
            "params": {}
        }"#;

        let req: JsonRpcRequest = serde_json::from_str(req_json).unwrap();
        assert!(req.extract_tool_call().is_none());
    }

    #[test]
    fn test_permission_denied_error_format() {
        let err = JsonRpcError::permission_denied(
            json!(42),
            "read_file",
            "Sensitive credentials access forbidden",
        );

        assert_eq!(err.id, json!(42));
        assert_eq!(err.error.code, -32001);
        assert!(err.error.message.contains("Aegis-MCP Security Alert"));
        assert!(err.error.message.contains("Sensitive credentials access forbidden"));

        let line = err.to_json_line().unwrap();
        assert!(line.ends_with('\n'));
    }
}
