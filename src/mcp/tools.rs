//! MCP tool implementations — maps tool calls to graph queries.

use serde_json::{json, Value};

use super::types::{ToolDefinition, ToolsCallResult};
use crate::graph::CodeGraph;
use crate::query::{anchor_dependencies, anchor_file_symbols, anchor_search, anchor_stats, Query};

/// Return the list of all available tools with their JSON schemas.
pub fn list_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "anchor_search".to_string(),
            description: "Search for code symbols (functions, classes, structs, etc.) by name. \
                Returns matching symbols with their source code, file location, \
                call relationships, and imports."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Symbol name to search for (e.g., 'login', 'UserService')"
                    },
                    "kind": {
                        "type": "string",
                        "description": "Optional: filter by symbol kind (function, method, struct, class, interface, enum, trait, type, constant, module, import, impl, variable)",
                        "enum": ["function", "method", "struct", "class", "interface", "enum", "trait", "type", "constant", "module", "import", "impl", "variable"]
                    },
                    "file": {
                        "type": "string",
                        "description": "Optional: filter by file path substring"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results to return (default: 5)",
                        "default": 5
                    }
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "anchor_dependencies".to_string(),
            description: "Find what a symbol depends on (calls, references) and what depends \
                on it (callers, referrers). Shows the dependency graph around a symbol."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "symbol": {
                        "type": "string",
                        "description": "The symbol name to analyze dependencies for"
                    }
                },
                "required": ["symbol"]
            }),
        },
        ToolDefinition {
            name: "anchor_stats".to_string(),
            description: "Get statistics about the code graph: file count, symbol count, \
                edge count, and unique symbol names."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolDefinition {
            name: "anchor_file_symbols".to_string(),
            description: "List all symbols defined in a specific file. Returns functions, \
                classes, structs, methods, constants, etc. with their source code."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file": {
                        "type": "string",
                        "description": "File path to list symbols for (e.g., 'src/main.rs')"
                    }
                },
                "required": ["file"]
            }),
        },
    ]
}

/// Dispatch a tool call to the appropriate handler.
pub fn call_tool(graph: &CodeGraph, name: &str, arguments: &Value) -> ToolsCallResult {
    match name {
        "anchor_search" => handle_search(graph, arguments),
        "anchor_dependencies" => handle_dependencies(graph, arguments),
        "anchor_stats" => handle_stats(graph),
        "anchor_file_symbols" => handle_file_symbols(graph, arguments),
        _ => ToolsCallResult::error(format!("Unknown tool: {}", name)),
    }
}

fn handle_search(graph: &CodeGraph, args: &Value) -> ToolsCallResult {
    let query_str = match args.get("query").and_then(|v| v.as_str()) {
        Some(q) => q,
        None => return ToolsCallResult::error("Missing required parameter: query".to_string()),
    };

    let kind = args.get("kind").and_then(|v| v.as_str()).map(String::from);
    let file = args.get("file").and_then(|v| v.as_str()).map(String::from);

    let query = if kind.is_some() || file.is_some() {
        Query::Structured {
            symbol: query_str.to_string(),
            kind,
            file,
        }
    } else {
        Query::Simple(query_str.to_string())
    };

    let response = anchor_search(graph, query);
    let json = serde_json::to_string_pretty(&response).unwrap_or_default();
    ToolsCallResult::text(json)
}

fn handle_dependencies(graph: &CodeGraph, args: &Value) -> ToolsCallResult {
    let symbol = match args.get("symbol").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return ToolsCallResult::error("Missing required parameter: symbol".to_string()),
    };

    let response = anchor_dependencies(graph, symbol);
    let json = serde_json::to_string_pretty(&response).unwrap_or_default();
    ToolsCallResult::text(json)
}

fn handle_stats(graph: &CodeGraph) -> ToolsCallResult {
    let response = anchor_stats(graph);
    let json = serde_json::to_string_pretty(&response).unwrap_or_default();
    ToolsCallResult::text(json)
}

fn handle_file_symbols(graph: &CodeGraph, args: &Value) -> ToolsCallResult {
    let file = match args.get("file").and_then(|v| v.as_str()) {
        Some(f) => f,
        None => return ToolsCallResult::error("Missing required parameter: file".to_string()),
    };

    let response = anchor_file_symbols(graph, file);
    let json = serde_json::to_string_pretty(&response).unwrap_or_default();
    ToolsCallResult::text(json)
}
