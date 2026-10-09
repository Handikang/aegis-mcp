use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "aegis")]
#[command(author = "Aegis Contributors")]
#[command(version = "0.1.0")]
#[command(
    about = "🛡️ The Blazingly Fast Security Firewall & Transparent Proxy for Model Context Protocol",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run transparent reverse proxy intercepting an upstream MCP command
    Proxy {
        /// The upstream command executable to spawn (e.g. npx, python, node)
        #[arg(required = true)]
        cmd: String,

        /// Arguments passed to the upstream command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Automatically wrap installed MCP configurations (Claude Desktop, Cursor)
    Wrap {
        /// Target specific config path (defaults to auto-discovering all supported clients)
        #[arg(short, long)]
        path: Option<String>,

        /// Only inspect configurations without applying modifications
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Restore original configurations from .aegis_bak backups
    Unwrap {
        /// Target specific config path
        #[arg(short, long)]
        path: Option<String>,
    },

    /// Run the 15-second terminal exploit simulation to verify firewall interception
    SimulateAttack,

    /// List active built-in security rules and detection patterns
    Rules,
}
