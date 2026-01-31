//! Anchor MCP Server — code intelligence for AI agents.
//!
//! Runs a JSON-RPC 2.0 server over STDIO that exposes the code graph
//! through the Model Context Protocol (MCP).
//!
//! Usage:
//!   anchor-mcp [project_root]
//!
//! If no project root is given, uses the current working directory.
//! The server watches for file changes and updates the graph in real-time.

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use tracing::info;

fn main() {
    // Initialize tracing to stderr (MCP uses stdout for protocol)
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Determine project root
    let project_root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    info!(root = %project_root.display(), "Anchor MCP server starting");

    // Load config
    let anchor_dir = project_root.join(".anchor");
    let config_path = anchor_dir.join("config.toml");
    let config = anchor::config::AnchorConfig::load(&config_path);

    // Try to load cached graph, or build fresh
    let cache_path = config.resolve_cache_path(&anchor_dir);
    let graph = load_or_build(&project_root, &cache_path);

    // Wrap in Arc<RwLock> for concurrent access
    let graph = Arc::new(RwLock::new(graph));

    // Start file watcher
    let _watcher = match anchor::watcher::start_watching(
        &project_root,
        Arc::clone(&graph),
        0, // use default debounce
    ) {
        Ok(handle) => {
            info!("file watcher active — graph updates in real-time");
            Some(handle)
        }
        Err(e) => {
            tracing::warn!(error = %e, "file watcher failed to start — graph will be static");
            None
        }
    };

    info!("MCP server ready — waiting for JSON-RPC requests on stdin");

    // Run the MCP server loop (blocks until stdin closes)
    anchor::mcp::server::run(Arc::clone(&graph));

    // Save graph on clean shutdown
    if let Ok(g) = graph.read() {
        if let Err(e) = g.save(&cache_path) {
            tracing::warn!(error = %e, "failed to save graph on shutdown");
        } else {
            info!("graph saved on shutdown");
        }
    };
}

/// Load graph from cache if available, otherwise build from source.
fn load_or_build(project_root: &Path, cache_path: &Path) -> anchor::CodeGraph {
    // Try loading from cache
    if cache_path.exists() {
        info!(cache = %cache_path.display(), "loading cached graph");
        match anchor::CodeGraph::load(cache_path) {
            Ok(graph) => {
                info!("graph loaded from cache");
                return graph;
            }
            Err(e) => {
                tracing::warn!(error = %e, "cache load failed, rebuilding");
            }
        }
    }

    // Build fresh
    let graph = anchor::build_graph(project_root);

    // Try to save to cache
    if let Some(parent) = cache_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = graph.save(cache_path) {
        tracing::warn!(error = %e, "failed to cache graph");
    }

    graph
}
