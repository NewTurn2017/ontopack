use anyhow::{anyhow, Result};
use pack_core::pack::{AddOutcome, Pack};
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
            "tools/call" => Ok(Some(response(id, self.call_tool(request.get("params"))))),
            other => Ok(Some(error_response(
                id,
                -32601,
                format!("unknown method: {other}"),
            ))),
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
        tool_schema(
            "search",
            "Search citation-ready source cards in the current pack",
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "k": { "type": "integer", "minimum": 1, "default": 10 },
                    "type": { "type": "string" },
                    "mode": { "type": "string", "enum": ["keyword", "vector", "hybrid"], "default": "keyword" }
                },
                "required": ["query"]
            }),
        ),
        tool_schema(
            "ask",
            "Return citation-ready context blocks for a question",
            json!({
                "type": "object",
                "properties": {
                    "question": { "type": "string" },
                    "k": { "type": "integer", "minimum": 1, "default": 5 }
                },
                "required": ["question"]
            }),
        ),
        tool_schema(
            "related",
            "Find notes related to a note id",
            json!({
                "type": "object",
                "properties": {
                    "note_id": { "type": "string" },
                    "depth": { "type": "integer", "minimum": 1, "default": 1 }
                },
                "required": ["note_id"]
            }),
        ),
        tool_schema(
            "add",
            "Add content or a file path to the pack",
            json!({
                "type": "object",
                "properties": {
                    "content": { "type": "string" },
                    "path": { "type": "string" },
                    "type": { "type": "string", "default": "note" },
                    "title": { "type": "string" },
                    "tags": { "type": "array", "items": { "type": "string" } }
                }
            }),
        ),
        tool_schema(
            "timeline",
            "List notes by created date",
            json!({
                "type": "object",
                "properties": {
                    "from": { "type": "string" },
                    "to": { "type": "string" },
                    "type": { "type": "string" },
                    "k": { "type": "integer", "minimum": 1, "default": 20 }
                }
            }),
        ),
    ]
}

fn tool_schema(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

fn response(id: Option<Value>, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id.unwrap_or(Value::Null), "result": result })
}

impl McpServer {
    fn call_tool(&self, params: Option<&Value>) -> Value {
        let Some(name) = params.and_then(|p| p.get("name")).and_then(Value::as_str) else {
            return tool_error("tools/call requires params.name");
        };
        let arguments = params
            .and_then(|p| p.get("arguments"))
            .cloned()
            .unwrap_or_else(|| json!({}));
        match name {
            "search" => self.tool_search(&arguments),
            "ask" => self.tool_ask(&arguments),
            "related" => self.tool_related(&arguments),
            "add" => self.tool_add(&arguments),
            "timeline" => self.tool_timeline(&arguments),
            other => tool_error(format!("unknown tool: {other}")),
        }
    }

    fn tool_search(&self, arguments: &Value) -> Value {
        let Some(query) = arguments.get("query").and_then(Value::as_str) else {
            return tool_error("search requires query");
        };
        let k = read_k(arguments, 10);
        let note_type_filter = arguments.get("type").and_then(Value::as_str);
        let mode = arguments
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("keyword");
        if mode != "keyword" {
            return tool_error("MCP vector/hybrid search requires a real embedding provider; use CLI real-embed path first");
        }
        match self.pack.search_keyword_chunks(query, k) {
            Ok(mut hits) => {
                if let Some(note_type) = note_type_filter {
                    hits.retain(|hit| hit.note_type == note_type);
                }
                tool_json(json!({
                    "query": query,
                    "mode": "keyword",
                    "hits": hits.into_iter().map(search_hit_json).collect::<Vec<_>>()
                }))
            }
            Err(err) => tool_error(err.to_string()),
        }
    }

    fn tool_ask(&self, arguments: &Value) -> Value {
        let Some(question) = arguments.get("question").and_then(Value::as_str) else {
            return tool_error("ask requires question");
        };
        let k = read_k(arguments, 5);
        match self.pack.search_keyword_chunks(question, k) {
            Ok(hits) => tool_json(json!({
                "question": question,
                "answer_mode": "external_llm_required",
                "instruction": "Use context_blocks to synthesize an answer with citations outside deterministic pack-core.",
                "context_blocks": hits.into_iter().map(search_hit_json).collect::<Vec<_>>()
            })),
            Err(err) => tool_error(err.to_string()),
        }
    }

    fn tool_related(&self, arguments: &Value) -> Value {
        let Some(note_id) = arguments.get("note_id").and_then(Value::as_str) else {
            return tool_error("related requires note_id");
        };
        let depth = arguments
            .get("depth")
            .and_then(Value::as_u64)
            .and_then(|v| usize::try_from(v).ok())
            .filter(|v| *v > 0)
            .unwrap_or(1);
        match self.pack.related_notes(note_id, depth) {
            Ok(related) => tool_json(json!({
                "note_id": note_id,
                "depth": depth,
                "related": related.into_iter().map(|note| json!({
                    "id": note.id,
                    "title": note.title,
                    "note_type": note.note_type,
                    "path": note.path.to_string_lossy(),
                    "depth": note.depth
                })).collect::<Vec<_>>()
            })),
            Err(err) => tool_error(err.to_string()),
        }
    }

    fn tool_add(&self, arguments: &Value) -> Value {
        let note_type = arguments
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("note");
        if let Some(content) = arguments.get("content").and_then(Value::as_str) {
            let title = arguments
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("untitled");
            let tags = read_tags(arguments);
            return match self
                .pack
                .add_content_note(title, content, note_type, tags.as_slice())
            {
                Ok(path) => tool_json(json!({
                    "added": {
                        "kind": "content",
                        "path": path.to_string_lossy()
                    }
                })),
                Err(err) => tool_error(err.to_string()),
            };
        }
        if let Some(path) = arguments.get("path").and_then(Value::as_str) {
            return match self.pack.add_file(Path::new(path), note_type) {
                Ok(AddOutcome::Note { path }) => tool_json(json!({
                    "added": {
                        "kind": "note",
                        "path": path.to_string_lossy()
                    }
                })),
                Ok(AddOutcome::AssetWithSidecar {
                    asset_path,
                    note_path,
                }) => tool_json(json!({
                    "added": {
                        "kind": "asset",
                        "asset_path": asset_path.to_string_lossy(),
                        "note_path": note_path.to_string_lossy()
                    }
                })),
                Err(err) => tool_error(err.to_string()),
            };
        }
        tool_error("add requires content or path")
    }

    fn tool_timeline(&self, arguments: &Value) -> Value {
        let from = arguments.get("from").and_then(Value::as_str);
        let to = arguments.get("to").and_then(Value::as_str);
        let note_type = arguments.get("type").and_then(Value::as_str);
        let k = read_k(arguments, 20);
        match self.pack.timeline_notes(from, to, note_type, k) {
            Ok(notes) => tool_json(json!({
                "notes": notes.into_iter().map(|note| json!({
                    "id": note.id,
                    "title": note.title,
                    "note_type": note.note_type,
                    "path": note.path.to_string_lossy(),
                    "created": note.created
                })).collect::<Vec<_>>()
            })),
            Err(err) => tool_error(err.to_string()),
        }
    }
}

fn read_k(arguments: &Value, default: usize) -> usize {
    arguments
        .get("k")
        .and_then(Value::as_u64)
        .and_then(|v| usize::try_from(v).ok())
        .filter(|v| *v > 0)
        .unwrap_or(default)
}

fn read_tags(arguments: &Value) -> Vec<String> {
    arguments
        .get("tags")
        .and_then(Value::as_array)
        .map(|tags| {
            tags.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn search_hit_json(hit: pack_core::search::SearchHit) -> Value {
    json!({
        "note_id": hit.note_id,
        "chunk_id": hit.chunk_id,
        "title": hit.title,
        "note_type": hit.note_type,
        "snippet": hit.snippet,
        "path": hit.path,
        "score": hit.score,
        "rank_source": rank_source_label(hit.rank_source)
    })
}

fn rank_source_label(source: pack_core::search::RankSource) -> &'static str {
    match source {
        pack_core::search::RankSource::Keyword => "keyword",
        pack_core::search::RankSource::Vector => "vector",
        pack_core::search::RankSource::Hybrid => "hybrid",
    }
}

fn tool_json(value: Value) -> Value {
    json!({
        "content": [{ "type": "text", "text": value.to_string() }],
        "isError": false
    })
}

fn tool_error(message: impl Into<String>) -> Value {
    tool_error_value(message.into())
}

fn tool_error_value(message: String) -> Value {
    json!({
        "content": [{ "type": "text", "text": json!({ "error": message }).to_string() }],
        "isError": true
    })
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

    #[test]
    fn search_tool_returns_source_cards() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(
            root.join("notes/hook.md"),
            "---\ntype: prompt\ntitle: 썸네일 훅\n---\n클릭을 부르는 훅 카피.",
        )
        .unwrap();
        let pack = Pack::open(&root).unwrap();
        pack.build_index().unwrap();
        let server = McpServer::open(&root).unwrap();

        let result = call_tool(&server, "search", json!({ "query": "훅", "k": 5 }));
        assert_eq!(result["hits"][0]["note_id"], "hook");
        assert_eq!(result["hits"][0]["chunk_id"], "hook#0000");
        assert_eq!(result["hits"][0]["rank_source"], "keyword");
        assert!(result["hits"][0]["snippet"]
            .as_str()
            .unwrap()
            .contains("클릭을 부르는 훅"));
    }

    #[test]
    fn ask_tool_returns_context_blocks() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(
            root.join("notes/hook.md"),
            "---\ntitle: 썸네일 훅\n---\n클릭을 부르는 훅 카피.",
        )
        .unwrap();
        let pack = Pack::open(&root).unwrap();
        pack.build_index().unwrap();
        let server = McpServer::open(&root).unwrap();

        let result = call_tool(&server, "ask", json!({ "question": "훅 자료?", "k": 3 }));
        assert_eq!(result["question"], "훅 자료?");
        assert_eq!(result["answer_mode"], "external_llm_required");
        assert_eq!(result["context_blocks"][0]["note_id"], "hook");
        assert!(result["context_blocks"][0]["snippet"]
            .as_str()
            .unwrap()
            .contains("클릭을 부르는 훅"));
    }

    #[test]
    fn related_tool_returns_linked_notes() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(root.join("notes/a.md"), "A [[b]]").unwrap();
        std::fs::write(root.join("notes/b.md"), "---\ntitle: B\n---\nB").unwrap();
        let server = McpServer::open(&root).unwrap();

        let result = call_tool(&server, "related", json!({ "note_id": "a", "depth": 1 }));
        assert_eq!(result["related"][0]["id"], "b");
        assert_eq!(result["related"][0]["depth"], 1);
    }

    #[test]
    fn timeline_tool_returns_filtered_notes() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(
            root.join("notes/new.md"),
            "---\ntype: prompt\ntitle: New\ncreated: 2026-02-01\n---\nnew",
        )
        .unwrap();
        std::fs::write(
            root.join("notes/img.md"),
            "---\ntype: image\ntitle: Img\ncreated: 2026-03-01\n---\nimg",
        )
        .unwrap();
        let server = McpServer::open(&root).unwrap();

        let result = call_tool(&server, "timeline", json!({ "type": "prompt", "k": 10 }));
        assert_eq!(result["notes"][0]["id"], "new");
        assert_eq!(result["notes"][0]["created"], "2026-02-01");
    }

    #[test]
    fn add_tool_adds_content_note() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        let server = McpServer::open(&root).unwrap();

        let result = call_tool(
            &server,
            "add",
            json!({
                "title": "강의 훅",
                "content": "본문",
                "type": "prompt",
                "tags": ["lecture"]
            }),
        );
        assert_eq!(result["added"]["kind"], "content");
        assert!(root.join("notes/강의 훅.md").exists());
    }

    fn call_tool(server: &McpServer, name: &str, arguments: Value) -> Value {
        let response = server
            .handle(json!({
                "jsonrpc": "2.0",
                "id": 99,
                "method": "tools/call",
                "params": { "name": name, "arguments": arguments }
            }))
            .unwrap()
            .unwrap();
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        serde_json::from_str(text).unwrap()
    }
}
