use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct WrapReport {
    pub wrapped_configs: Vec<PathBuf>,
    pub total_servers_wrapped: usize,
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct UnwrapReport {
    pub restored_configs: Vec<PathBuf>,
}

pub struct ConfigWrapper;

impl ConfigWrapper {
    /// Discovers common MCP client configuration paths across platforms.
    pub fn discover_config_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // 1. Claude Desktop
        #[cfg(target_os = "windows")]
        if let Ok(appdata) = std::env::var("APPDATA") {
            paths.push(PathBuf::from(appdata).join("Claude").join("claude_desktop_config.json"));
        }

        #[cfg(target_os = "macos")]
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Library/Application Support/Claude/claude_desktop_config.json"));
        }

        #[cfg(target_os = "linux")]
        if let Some(config_dir) = dirs::config_dir() {
            paths.push(config_dir.join("Claude/claude_desktop_config.json"));
        }

        // 2. Cursor
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".cursor").join("mcp.json"));
            paths.push(home.join(".codeium").join("windsurf").join("mcp_config.json"));
        }

        // Filter for existing files
        paths.into_iter().filter(|p| p.exists()).collect()
    }

    /// Wraps an MCP configuration file to route its servers through `aegis proxy`.
    pub fn wrap_file(path: &Path, aegis_bin: &str) -> Result<usize> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file at {:?}", path))?;

        let mut json: Value = serde_json::from_str(&content)
            .with_context(|| format!("Invalid JSON in config file at {:?}", path))?;

        let servers = json.get_mut("mcpServers").and_then(|v| v.as_object_mut());
        let Some(servers) = servers else {
            return Ok(0);
        };

        let mut wrapped_count = 0;

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
    fn test_wrap_and_unwrap_cycle() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("mcp.json");

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
        let cmd = wrapped_val["mcpServers"]["filesystem"]["command"].as_str().unwrap();
        let args: Vec<&str> = wrapped_val["mcpServers"]["filesystem"]["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();

        assert_eq!(cmd, "aegis");
        assert_eq!(args, vec!["proxy", "--", "npx", "-y", "@modelcontextprotocol/server-filesystem", "/tmp"]);

        // 2. Unwrap
        let restored = ConfigWrapper::unwrap_file(&config_path).unwrap();
        assert!(restored);

        let restored_content = fs::read_to_string(&config_path).unwrap();
        let restored_val: Value = serde_json::from_str(&restored_content).unwrap();
        assert_eq!(
            restored_val["mcpServers"]["filesystem"]["command"].as_str().unwrap(),
            "npx"
        );
    }
}
