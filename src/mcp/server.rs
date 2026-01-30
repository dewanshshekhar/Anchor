//! MCP JSON-RPC 2.0 server — reads requests from stdin, writes responses to stdout.
//!
//! The MCP protocol uses newline-delimited JSON over STDIO.
//! Tracing output goes to stderr so it doesn't interfere with the protocol.

use std::io::{self, BufRead, Write};
use std::sync::{Arc, RwLock};

use serde_json::Value;
use tracing::{debug, error, info, warn};

use super::tools;
use super::types::*;
use crate::graph::CodeGraph;

/// Run the MCP server loop, reading JSON-RPC from stdin and writing to stdout.
///
/// Accepts a shared graph reference so the watcher can update it concurrently.
pub fn run(graph: Arc<RwLock<CodeGraph>>) {
    info!("MCP server starting");

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                error!(error = %e, "failed to read stdin");
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        debug!(request = %trimmed, "received request");

        let request: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, "invalid JSON-RPC request");
                let response = JsonRpcResponse::error(
                    None,
                    -32700,
                    format!("Parse error: {}", e),
                );
                write_response(&mut stdout, &response);
                continue;
            }
        };

        let response = handle_request(&graph, &request);

        if let Some(resp) = response {
            write_response(&mut stdout, &resp);
        }
    }

    info!("MCP server shutting down");
}

/// Handle a single JSON-RPC request and return a response (or None for notifications).
fn handle_request(
    graph: &Arc<RwLock<CodeGraph>>,
    request: &JsonRpcRequest,
) -> Option<JsonRpcResponse> {
    let id = request.id.clone();

    match request.method.as_str() {
        "initialize" => {
            info!("client initializing");
            let result = InitializeResult {
                protocol_version: "2024-11-05".to_string(),
                capabilities: ServerCapabilities {
                    tools: ToolCapability {},
                },
                server_info: ServerInfo {
                    name: "anchor".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                },
            };
            Some(JsonRpcResponse::success(
                id,
                serde_json::to_value(result).unwrap(),
            ))
        }

        "notifications/initialized" => {
            info!("client initialized");
            None // Notifications don't get responses
        }

        "tools/list" => {
            debug!("listing tools");
            let result = ToolsListResult {
                tools: tools::list_tools(),
            };
            Some(JsonRpcResponse::success(
                id,
                serde_json::to_value(result).unwrap(),
            ))
        }

        "tools/call" => {
            let params: ToolsCallParams = match serde_json::from_value(request.params.clone()) {
                Ok(p) => p,
                Err(e) => {
                    return Some(JsonRpcResponse::error(
                        id,
                        -32602,
                        format!("Invalid params: {}", e),
                    ));
                }
            };

            debug!(tool = %params.name, "calling tool");

            // Acquire read lock on the graph for tool execution
            let graph_guard = match graph.read() {
                Ok(g) => g,
                Err(e) => {
                    return Some(JsonRpcResponse::error(
                        id,
                        -32603,
                        format!("Graph lock error: {}", e),
                    ));
                }
            };

            let result = tools::call_tool(&graph_guard, &params.name, &params.arguments);
            Some(JsonRpcResponse::success(
                id,
                serde_json::to_value(result).unwrap(),
            ))
        }

        "ping" => {
            Some(JsonRpcResponse::success(id, Value::Object(Default::default())))
        }

        _ => {
            warn!(method = %request.method, "unknown method");
            Some(JsonRpcResponse::error(
                id,
                -32601,
                format!("Method not found: {}", request.method),
            ))
        }
    }
}

/// Write a JSON-RPC response to stdout (newline-delimited).
fn write_response(stdout: &mut impl Write, response: &JsonRpcResponse) {
    let json = serde_json::to_string(response).unwrap_or_default();
    debug!(response = %json, "sending response");
    let _ = writeln!(stdout, "{}", json);
    let _ = stdout.flush();
}
