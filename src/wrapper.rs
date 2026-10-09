use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DiscoveredClient {
    pub client_name: &'static str,
    pub path: PathBuf,
}

pub struct ConfigWrapper;

impl ConfigWrapper {
    /// Discovers MCP configurations across a wide variety of IDEs, agent harnesses, and tools.
    pub fn discover_clients() -> Vec<DiscoveredClient> {
        let mut candidates: Vec<(&'static str, PathBuf)> = Vec::new();

        // 1. Google Antigravity & Antigravity IDE
        if let Some(home) = dirs::home_dir() {
            candidates.push(("Antigravity", home.join(".gemini/antigravity/mcp_config.json")));
            candidates.push(("Antigravity IDE", home.join(".gemini/antigravity-ide/mcp_config.json")));
            candidates.push(("Antigravity Config", home.join(".gemini/config/mcp_config.json")));
        }

        // 2. Claude Desktop & Claude Code CLI
        #[cfg(target_os = "windows")]
        if let Ok(appdata) = std::env::var("APPDATA") {
            candidates.push(("Claude Desktop", PathBuf::from(appdata).join("Claude/claude_desktop_config.json")));
        }
        #[cfg(target_os = "macos")]
        if let Some(home) = dirs::home_dir() {
            candidates.push(("Claude Desktop", home.join("Library/Application Support/Claude/claude_desktop_config.json")));
        }
        #[cfg(target_os = "linux")]
        if let Some(config_dir) = dirs::config_dir() {
            candidates.push(("Claude Desktop", config_dir.join("Claude/claude_desktop_config.json")));
        }
        if let Some(home) = dirs::home_dir() {
            candidates.push(("Claude Code CLI", home.join(".claude/.mcp.json")));
        }

        // 3. Cursor
        if let Some(home) = dirs::home_dir() {
            candidates.push(("Cursor", home.join(".cursor/mcp.json")));
        }

        // 4. Windsurf
        if let Some(home) = dirs::home_dir() {
            candidates.push(("Windsurf", home.join(".codeium/windsurf/mcp_config.json")));
        }
        #[cfg(target_os = "windows")]
        if let Ok(appdata) = std::env::var("APPDATA") {
            candidates.push(("Windsurf", PathBuf::from(appdata).join("Codeium/windsurf/mcp_config.json")));
        }

        // 5. Codex, DeepSeek Harness, Agent Harnesses
        if let Some(home) = dirs::home_dir() {
            candidates.push(("Codex CLI", home.join(".codex/mcp.json")));
            candidates.push(("Codex Config", home.join(".codex/config.json")));
            candidates.push(("DeepSeek Harness", home.join(".deepseek/mcp.json")));
            candidates.push(("DeepSeek Config", home.join(".deepseek/config.json")));
            candidates.push(("Agent Harness", home.join(".harness/mcp.json")));
        }

        // 6. VS Code Extensions (Cline, Roo Code, Continue.dev)
        if let Some(home) = dirs::home_dir() {
            candidates.push(("Continue.dev", home.join(".continue/config.json")));
        }
        #[cfg(target_os = "windows")]
        if let Ok(appdata) = std::env::var("APPDATA") {
            let code_user = PathBuf::from(&appdata).join("Code/User/globalStorage");
            candidates.push(("Cline", code_user.join("saoudrizwan.claude-dev/settings/cline_mcp_settings.json")));
            candidates.push(("Roo Code", code_user.join("rooveterinaryinc.roo-cline/settings/cline_mcp_settings.json")));
        }
        #[cfg(target_os = "macos")]
        if let Some(home) = dirs::home_dir() {
            let code_user = home.join("Library/Application Support/Code/User/globalStorage");
            candidates.push(("Cline", code_user.join("saoudrizwan.claude-dev/settings/cline_mcp_settings.json")));
            candidates.push(("Roo Code", code_user.join("rooveterinaryinc.roo-cline/settings/cline_mcp_settings.json")));
        }
        #[cfg(target_os = "linux")]
        if let Some(config_dir) = dirs::config_dir() {
            let code_user = config_dir.join("Code/User/globalStorage");
            candidates.push(("Cline", code_user.join("saoudrizwan.claude-dev/settings/cline_mcp_settings.json")));
            candidates.push(("Roo Code", code_user.join("rooveterinaryinc.roo-cline/settings/cline_mcp_settings.json")));
        }

        // 7. 国内主流开发工具 (通义灵码, Qoder, WorkBuddy)
        if let Some(home) = dirs::home_dir() {
            candidates.push(("Tongyi Lingma", home.join(".lingma/lingma_mcp.json")));
            candidates.push(("Qoder", home.join(".qoder/shared_client/mcp.json")));
            candidates.push(("Qoder CN", home.join(".qoder-cn/shared_client/mcp.json")));
            candidates.push(("WorkBuddy", home.join(".workbuddy/mcp.json")));
        }

        // 8. Zed Editor
        if let Some(home) = dirs::home_dir() {
            candidates.push(("Zed", home.join(".config/zed/settings.json")));
        }
        #[cfg(target_os = "windows")]
        if let Ok(appdata) = std::env::var("APPDATA") {
            candidates.push(("Zed", PathBuf::from(appdata).join("Zed/settings.json")));
        }

        // 9. Current Workspace Project Configs
        if let Ok(cwd) = std::env::current_dir() {
            candidates.push(("Workspace .mcp.json", cwd.join(".mcp.json")));
            candidates.push(("Workspace mcp.json", cwd.join("mcp.json")));
            candidates.push(("Workspace mcp_config.json", cwd.join("mcp_config.json")));
            candidates.push(("Workspace Cursor MCP", cwd.join(".cursor/mcp.json")));
        }

        // Filter and deduplicate existing files with valid content
        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for (client_name, path) in candidates {
            if path.exists() && seen.insert(path.clone()) {
                // Ensure file is non-empty
                if let Ok(meta) = fs::metadata(&path) {
                    if meta.len() > 0 {
                        results.push(DiscoveredClient { client_name, path });
                    }
                }
            }
        }

        results
    }

    /// Wraps an MCP configuration file to route its servers through `aegis proxy`.
    /// Supports both camelCase ("mcpServers") and snake_case ("mcp_servers") schemas.
    pub fn wrap_file(path: &Path, aegis_bin: &str) -> Result<usize> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file at {:?}", path))?;

        let mut json: Value = serde_json::from_str(&content)
            .with_context(|| format!("Invalid JSON in config file at {:?}", path))?;

        let mut wrapped_count = 0;
        let server_keys = ["mcpServers", "mcp_servers"];

        for key in server_keys {
            if let Some(servers) = json.get_mut(key).and_then(|v| v.as_object_mut()) {
                for (_name, server_val) in servers.iter_mut() {
                    let Some(server_obj) = server_val.as_object_mut() else {
                        continue;
                    };

                    let Some(orig_cmd) = server_obj.get("command").and_then(|v| v.as_str()) else {
                        continue;
                    };

                    // Avoid double wrapping
                    if orig_cmd == "aegis" || orig_cmd.ends_with("aegis.exe") || orig_cmd.ends_with("/aegis") {
                        continue;
                    }

                    let mut orig_args: Vec<String> = server_obj
                        .get("args")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|val| val.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    let mut new_args = vec!["proxy".to_string(), "--".to_string(), orig_cmd.to_string()];
                    new_args.append(&mut orig_args);

                    server_obj.insert("command".to_string(), Value::String(aegis_bin.to_string()));
                    server_obj.insert(
                        "args".to_string(),
                        Value::Array(new_args.into_iter().map(Value::String).collect()),
                    );

                    wrapped_count += 1;
                }
            }
        }

        if wrapped_count > 0 {
            // Save backup file
            let backup_path = path.with_extension("json.aegis_bak");
            fs::write(&backup_path, &content)
                .with_context(|| format!("Failed to create backup at {:?}", backup_path))?;

            // Write wrapped config
            let formatted_json = serde_json::to_string_pretty(&json)?;
            fs::write(path, formatted_json)
                .with_context(|| format!("Failed to write wrapped config at {:?}", path))?;
        }

        Ok(wrapped_count)
    }

    /// Unwraps an MCP configuration file by restoring from `.aegis_bak`.
    pub fn unwrap_file(path: &Path) -> Result<bool> {
        let backup_path = path.with_extension("json.aegis_bak");
        if !backup_path.exists() {
            return Ok(false);
        }

        let backup_content = fs::read_to_string(&backup_path)?;
        fs::write(path, backup_content)?;
        let _ = fs::remove_file(backup_path);

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn test_wrap_and_unwrap_camel_case() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("mcp_config.json");

        let original_json = json!({
            "mcpServers": {
                "filesystem": {
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
                }
            }
        });

        fs::write(&config_path, serde_json::to_string_pretty(&original_json).unwrap()).unwrap();

        // 1. Wrap
        let wrapped_count = ConfigWrapper::wrap_file(&config_path, "aegis").unwrap();
        assert_eq!(wrapped_count, 1);

        let wrapped_content = fs::read_to_string(&config_path).unwrap();
        let wrapped_val: Value = serde_json::from_str(&wrapped_content).unwrap();
        assert_eq!(wrapped_val["mcpServers"]["filesystem"]["command"].as_str().unwrap(), "aegis");

        // 2. Unwrap
        let restored = ConfigWrapper::unwrap_file(&config_path).unwrap();
        assert!(restored);

        let restored_content = fs::read_to_string(&config_path).unwrap();
        let restored_val: Value = serde_json::from_str(&restored_content).unwrap();
        assert_eq!(restored_val["mcpServers"]["filesystem"]["command"].as_str().unwrap(), "npx");
    }

    #[test]
    fn test_wrap_and_unwrap_snake_case_deepseek_codex() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join(".mcp.json");

        let original_json = json!({
            "mcp_servers": {
                "codebase_analyzer": {
                    "command": "python",
                    "args": ["-m", "analyzer_server"]
                }
            }
        });

        fs::write(&config_path, serde_json::to_string_pretty(&original_json).unwrap()).unwrap();

        // 1. Wrap
        let wrapped_count = ConfigWrapper::wrap_file(&config_path, "aegis").unwrap();
        assert_eq!(wrapped_count, 1);

        let wrapped_content = fs::read_to_string(&config_path).unwrap();
        let wrapped_val: Value = serde_json::from_str(&wrapped_content).unwrap();
        assert_eq!(wrapped_val["mcp_servers"]["codebase_analyzer"]["command"].as_str().unwrap(), "aegis");

        // 2. Unwrap
        let restored = ConfigWrapper::unwrap_file(&config_path).unwrap();
        assert!(restored);
    }
}
