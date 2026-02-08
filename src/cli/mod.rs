//! CLI module for Anchor.
//!
//! Commands:
//! - Read/Search: search, read, context
//! - Write: write, edit (TODO: ACI-based)
//! - Parallel: plan
//! - System: build, stats, daemon

pub mod daemon;
pub mod plan;
pub mod read;
// pub mod write;  // TODO: Write operations not finalized yet

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "anchor")]
#[command(about = "Code Intelligence for AI Agents")]
#[command(override_help = HELP_TEXT)]
pub struct Cli {
    /// Project root directory (default: current directory)
    #[arg(short, long, default_value = ".")]
    pub root: PathBuf,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

const HELP_TEXT: &str = "
  █████╗ ███╗   ██╗ ██████╗██╗  ██╗ ██████╗ ██████╗
 ██╔══██╗████╗  ██║██╔════╝██║  ██║██╔═══██╗██╔══██╗
 ███████║██╔██╗ ██║██║     ███████║██║   ██║██████╔╝
 ██╔══██║██║╚██╗██║██║     ██╔══██║██║   ██║██╔══██╗
 ██║  ██║██║ ╚████║╚██████╗██║  ██║╚██████╔╝██║  ██║
 ╚═╝  ╚═╝╚═╝  ╚═══╝ ╚═════╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝
        Code Intelligence for AI Agents

Start here:
  build                 Index codebase
  map                   Codebase map (modules + top symbols)
  map <scope>           Zoom into module

Query:
  context <symbol>      Code + callers + callees
  search <query>        Find symbols
  plan <file.json>      Batch read operations

Other:
  overview              Files + symbol counts
  stats                 Graph statistics

Options:
  -r, --root <PATH>     Project root (default: .)
";

#[derive(Subcommand)]
pub enum Commands {
    // ─── Query Commands ─────────────────────────────────────────────
    /// Get symbol context (code + callers + callees)
    Context {
        /// Symbol name to query
        query: String,

        /// Max results
        #[arg(short, long, default_value = "5")]
        limit: usize,
    },

    /// Search for symbols (lightweight: names, files, lines)
    Search {
        /// Symbol name to search for
        query: String,

        /// Regex pattern (Brzozowski derivatives - ReDoS safe)
        #[arg(short, long)]
        pattern: Option<String>,

        /// Max results
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    // ─── Parallel (1 command) ─────────────────────────────────────
    /// Execute parallel read operations from plan.json
    Plan {
        /// Path to plan JSON file
        file: String,
    },

    // ─── Write (hidden - not finalized) ──────────────────────────
    #[command(hide = true)]
    Write {
        path: String,
        content: String,
    },

    #[command(hide = true)]
    Edit {
        path: String,
        #[arg(short, long)]
        action: String,
        #[arg(short, long)]
        pattern: String,
        #[arg(short, long)]
        content: Option<String>,
    },

    // ─── Overview ─────────────────────────────────────────────────
    /// Compact codebase map for AI agents
    Map {
        /// Optional scope: zoom into specific module/directory
        scope: Option<String>,
    },

    /// Show codebase overview (files, structure, key symbols)
    Overview,

    // ─── System ───────────────────────────────────────────────────
    /// Build/rebuild the code graph
    Build,

    /// Show graph statistics
    Stats,

    // ─── Hidden Commands ─────────────────────────────────────────
    /// List all indexed files
    #[command(hide = true)]
    Files,

    /// Manage the anchor daemon
    #[command(hide = true)]
    Daemon {
        #[command(subcommand)]
        action: Option<daemon::DaemonAction>,
    },

    /// Update anchor to latest version
    #[command(hide = true)]
    Update,

    /// Uninstall anchor (runs shell script)
    #[command(hide = true)]
    Uninstall,

    /// Show version
    #[command(hide = true)]
    Version,
}

/// Print the ASCII banner (only for install/update)
pub fn print_banner() {
    println!(
        r#"
 █████╗ ███╗   ██╗ ██████╗██╗  ██╗ ██████╗ ██████╗
██╔══██╗████╗  ██║██╔════╝██║  ██║██╔═══██╗██╔══██╗
███████║██╔██╗ ██║██║     ███████║██║   ██║██████╔╝
██╔══██║██║╚██╗██║██║     ██╔══██║██║   ██║██╔══██╗
██║  ██║██║ ╚████║╚██████╗██║  ██║╚██████╔╝██║  ██║
╚═╝  ╚═╝╚═╝  ╚═══╝ ╚═════╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝

        Code Intelligence for AI Agents
"#
    );
}

/// Print usage help
pub fn print_usage() {
    print!("{}", HELP_TEXT);
}
