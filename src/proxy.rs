use anyhow::{Context, Result};
use colored::Colorize;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use crate::firewall::{FirewallEngine, Verdict};
use crate::protocol::{JsonRpcError, JsonRpcRequest};

pub struct StdioProxy {
    firewall: Arc<FirewallEngine>,
}

impl StdioProxy {
    pub fn new() -> Self {
        Self {
            firewall: Arc::new(FirewallEngine::new()),
        }
    }

    /// Runs the transparent reverse proxy between client stdin/stdout and upstream MCP server.
    pub async fn run(&self, upstream_cmd: &str, upstream_args: &[String]) -> Result<()> {
        let mut child = Command::new(upstream_cmd)
            .args(upstream_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("Failed to spawn upstream MCP server: '{}'", upstream_cmd))?;

        let child_stdin = child.stdin.take().expect("Failed to open child stdin");
        let child_stdout = child.stdout.take().expect("Failed to open child stdout");

        let firewall = Arc::clone(&self.firewall);

        // Task A: Client STDIN -> Upstream STDIN (with in-memory interception)
        let client_to_upstream = tokio::spawn(async move {
            let mut client_reader = BufReader::new(tokio::io::stdin()).lines();
            let mut child_writer = child_stdin;
            let mut client_writer = tokio::io::stdout();

            while let Ok(Some(line)) = client_reader.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }

                // Check if this line is a JSON-RPC tools/call request
                let mut should_block = None;

                if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&line) {
                    if let Some((tool_name, args)) = req.extract_tool_call() {
                        let verdict = firewall.evaluate(tool_name, args);
                        if let Verdict::Blocked { rule_id, severity, matched_content, description } = verdict {
                            should_block = Some((req.id.clone().unwrap_or(serde_json::Value::Null), tool_name.to_string(), rule_id, severity, matched_content, description));
                        }
                    }
                }

                if let Some((req_id, tool_name, rule_id, severity, matched_content, description)) = should_block {
                    // Display visual security banner to stderr
                    eprintln!("{}", "\n🚨 [Aegis-MCP] THREAT INTERCEPTED & MITIGATED!".bold().red());
                    eprintln!("  {}  : {}", "Tool Name".bold().white(), tool_name.yellow());
                    eprintln!("  {}  : {} ({})", "Triggered".bold().white(), rule_id.red().bold(), severity.red());
                    eprintln!("  {}  : {}", "Matched   ".bold().white(), matched_content.bright_red());
                    eprintln!("  {}  : {}", "Reason    ".bold().white(), description.bright_black());
                    eprintln!("  {}  : {}\n", "Action    ".bold().white(), "Blocked at proxy layer. Synthesizing safe response.".green());

                    // Synthesize error for client LLM
                    let reason = format!("Rule '{}' triggered on '{}': {}", rule_id, matched_content, description);
                    let err = JsonRpcError::permission_denied(req_id, &tool_name, &reason);

                    if let Ok(err_line) = err.to_json_line() {
                        let _ = client_writer.write_all(err_line.as_bytes()).await;
                        let _ = client_writer.flush().await;
                    }
                } else {
                    // Safe command: forward to upstream child
                    let mut payload = line.into_bytes();
                    payload.push(b'\n');
                    if child_writer.write_all(&payload).await.is_err() {
                        break;
                    }
                    let _ = child_writer.flush().await;
                }
            }
        });

        // Task B: Upstream STDOUT -> Client STDOUT (transparent passthrough)
        let upstream_to_client = tokio::spawn(async move {
            let mut child_reader = BufReader::new(child_stdout).lines();
            let mut client_writer = tokio::io::stdout();

            while let Ok(Some(line)) = child_reader.next_line().await {
                let mut payload = line.into_bytes();
                payload.push(b'\n');
                if client_writer.write_all(&payload).await.is_err() {
                    break;
                }
                let _ = client_writer.flush().await;
            }
        });

        tokio::select! {
            _ = client_to_upstream => {},
            _ = upstream_to_client => {},
            status = child.wait() => {
                let _ = status;
            }
        }

        Ok(())
    }
}
