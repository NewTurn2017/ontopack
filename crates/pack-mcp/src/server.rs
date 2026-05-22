use anyhow::{anyhow, Result};
use pack_core::pack::Pack;
use serde_json::{json, Value};
use std::path::Path;

pub struct McpServer {
    pack: Pack,
}

impl McpServer {
    pub fn open(pack_root: &Path) -> Result<Self> {
        Ok(Self {
            pack: Pack::open(pack_root)?,
        })
    }

    pub fn handle(&self, request: Value) -> Result<Option<Value>> {
        let Some(method) = request.get("method").and_then(Value::as_str) else {
            return Err(anyhow!("JSON-RPC request is missing method"));
        };
        let id = request.get("id").cloned();
        match method {
            "initialize" => Ok(Some(response(id, initialize_result()))),
            "notifications/initialized" => Ok(None),
            "tools/list" => Ok(Some(response(id, json!({ "tools": tool_schemas() })))),
            "tools/call" => Ok(Some(response(id, tool_call_placeholder()))),
            other => Ok(Some(error_response(id, -32601, format!("unknown method: {other}")))),
        }
    }
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "ontopack", "version": env!("CARGO_PKG_VERSION") }
    })
}

fn tool_schemas() -> Vec<Value> {
    vec![
        tool_schema("search", "Search citation-ready source cards in the current pack", json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" },
                "k": { "type": "integer", "minimum": 1, "default": 10 },
                "type": { "type": "string" },
                "mode": { "type": "string", "enum": ["keyword", "vector", "hybrid"], "default": "keyword" }
            },
            "required": ["query"]
        })),
        tool_schema("ask", "Return citation-ready context blocks for a question", json!({
            "type": "object",
            "properties": {
                "question": { "type": "string" },
                "k": { "type": "integer", "minimum": 1, "default": 5 }
            },
            "required": ["question"]
        })),
        tool_schema("related", "Find notes related to a note id", json!({
            "type": "object",
            "properties": {
                "note_id": { "type": "string" },
                "depth": { "type": "integer", "minimum": 1, "default": 1 }
            },
            "required": ["note_id"]
        })),
        tool_schema("add", "Add content or a file path to the pack", json!({
            "type": "object",
            "properties": {
                "content": { "type": "string" },
                "path": { "type": "string" },
                "type": { "type": "string", "default": "note" },
                "title": { "type": "string" },
                "tags": { "type": "array", "items": { "type": "string" } }
            }
        })),
        tool_schema("timeline", "List notes by created date", json!({
            "type": "object",
            "properties": {
                "from": { "type": "string" },
                "to": { "type": "string" },
                "type": { "type": "string" },
                "k": { "type": "integer", "minimum": 1, "default": 20 }
            }
        })),
    ]
}

fn tool_schema(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

fn tool_call_placeholder() -> Value {
    json!({
        "content": [{ "type": "text", "text": "tool implementation pending" }],
        "isError": true
    })
}

fn response(id: Option<Value>, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id.unwrap_or(Value::Null), "result": result })
}

fn error_response(id: Option<Value>, code: i64, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "error": { "code": code, "message": message }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pack_core::pack::Pack;
    use tempfile::tempdir;

    #[test]
    fn initialize_and_lists_tools() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        let server = McpServer::open(&root).unwrap();

        let init = server
            .handle(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))
            .unwrap()
            .unwrap();
        assert_eq!(init["result"]["serverInfo"]["name"], "ontopack");
        assert_eq!(init["result"]["capabilities"]["tools"], json!({}));

        let list = server
            .handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}))
            .unwrap()
            .unwrap();
        let names: Vec<_> = list["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, vec!["search", "ask", "related", "add", "timeline"]);
    }
}
