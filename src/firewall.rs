use regex::Regex;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Severity {
    Critical,
    High,
    Medium,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Critical => "CRITICAL",
            Severity::High => "HIGH",
            Severity::Medium => "MEDIUM",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SecurityRule {
    pub id: &'static str,
    pub description: &'static str,
    pub severity: Severity,
    pub pattern: Regex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Allowed,
    Blocked {
        rule_id: String,
        description: String,
        severity: String,
        matched_content: String,
    },
}

pub struct FirewallEngine {
    rules: Vec<SecurityRule>,
}

impl Default for FirewallEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl FirewallEngine {
    pub fn new() -> Self {
        let rules = vec![
            SecurityRule {
                id: "AEGIS-001-SSH-KEY",
                description: "Prohibits access to SSH private keys and authorized credentials",
                severity: Severity::Critical,
                pattern: Regex::new(r"(?i)(\.ssh[/\\]|id_rsa|id_ed25519|id_ecdsa|id_dsa)").unwrap(),
            },
            SecurityRule {
                id: "AEGIS-002-ENV-SECRETS",
                description: "Prohibits reading environment secret files (.env, .env.local)",
                severity: Severity::Critical,
                pattern: Regex::new(r"(?i)(\.env(\.(local|production|dev|staging))?)\b").unwrap(),
            },
            SecurityRule {
                id: "AEGIS-003-AWS-CREDENTIALS",
                description: "Prohibits reading AWS credentials or API keys",
                severity: Severity::Critical,
                pattern: Regex::new(r"(?i)(\.aws[/\\]credentials|AKIA[0-9A-Z]{16})").unwrap(),
            },
            SecurityRule {
                id: "AEGIS-004-SYSTEM-PASSWD",
                description: "Prohibits reading sensitive system credential files (/etc/shadow, /etc/passwd)",
                severity: Severity::Critical,
                pattern: Regex::new(r"(?i)(/etc/shadow|/etc/passwd|\bsam\b|SYSTEM32[/\\]config)").unwrap(),
            },
            SecurityRule {
                id: "AEGIS-005-DESTRUCTIVE-DELETE",
                description: "Prohibits recursive force deletion of root or home directories",
                severity: Severity::Critical,
                pattern: Regex::new(r"(?i)(rm\s+-[a-z]*r[a-z]*f[a-z]*\s+[/~]|rm\s+-[a-z]*f[a-z]*r[a-z]*\s+[/~]|format\s+[c-z]:)").unwrap(),
            },
            SecurityRule {
                id: "AEGIS-006-DATA-EXFILTRATION",
                description: "Prohibits data exfiltration pipeline (piping sensitive output to curl/wget/netcat)",
                severity: Severity::High,
                pattern: Regex::new(r"(?i)(\|\s*(curl|wget)\b.*(-d|--data)|nc\s+-[a-z]*e\b|bash\s+-i\s+>&)").unwrap(),
            },
        ];

        Self { rules }
    }

    /// Evaluates a tool call arguments payload against active security rules.
    pub fn evaluate(&self, _tool_name: &str, arguments: Option<&Value>) -> Verdict {
        let Some(args) = arguments else {
            return Verdict::Allowed;
        };

        // Extract all text content from JSON arguments for deep scanning
        let mut target_texts = Vec::new();
        Self::collect_strings(args, &mut target_texts);

        for text in target_texts {
            for rule in &self.rules {
                if let Some(m) = rule.pattern.find(&text) {
                    return Verdict::Blocked {
                        rule_id: rule.id.to_string(),
                        description: rule.description.to_string(),
                        severity: rule.severity.as_str().to_string(),
                        matched_content: m.as_str().to_string(),
                    };
                }
            }
        }

        Verdict::Allowed
    }

    fn collect_strings(val: &Value, out: &mut Vec<String>) {
        match val {
            Value::String(s) => out.push(s.clone()),
            Value::Array(arr) => {
                for item in arr {
                    Self::collect_strings(item, out);
                }
            }
            Value::Object(map) => {
                for (_, v) in map {
                    Self::collect_strings(v, out);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_block_ssh_key_access() {
        let firewall = FirewallEngine::new();
        let args = json!({
            "command": "cat ~/.ssh/id_rsa | base64"
        });

        let verdict = firewall.evaluate("bash_execute", Some(&args));
        match verdict {
            Verdict::Blocked { rule_id, severity, .. } => {
                assert_eq!(rule_id, "AEGIS-001-SSH-KEY");
                assert_eq!(severity, "CRITICAL");
            }
            Verdict::Allowed => panic!("Expected blocked for SSH key!"),
        }
    }

    #[test]
    fn test_block_env_secrets() {
        let firewall = FirewallEngine::new();
        let args = json!({
            "path": "/project/.env"
        });

        let verdict = firewall.evaluate("read_file", Some(&args));
        match verdict {
            Verdict::Blocked { rule_id, .. } => {
                assert_eq!(rule_id, "AEGIS-002-ENV-SECRETS");
            }
            Verdict::Allowed => panic!("Expected blocked for .env!"),
        }
    }

    #[test]
    fn test_block_destructive_rm() {
        let firewall = FirewallEngine::new();
        let args = json!({
            "command": "rm -rf / --no-preserve-root"
        });

        let verdict = firewall.evaluate("terminal", Some(&args));
        match verdict {
            Verdict::Blocked { rule_id, .. } => {
                assert_eq!(rule_id, "AEGIS-005-DESTRUCTIVE-DELETE");
            }
            Verdict::Allowed => panic!("Expected blocked for rm -rf!"),
        }
    }

    #[test]
    fn test_allow_safe_operations() {
        let firewall = FirewallEngine::new();
        let args = json!({
            "command": "git status && cargo check",
            "path": "src/main.rs"
        });

        let verdict = firewall.evaluate("bash_execute", Some(&args));
        assert_eq!(verdict, Verdict::Allowed);
    }
}
