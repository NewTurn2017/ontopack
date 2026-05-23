use crate::{api, viewer};
use anyhow::{anyhow, bail, Result};
use pack_core::pack::Pack;
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn to_http_bytes(&self) -> Vec<u8> {
        let reason = reason_phrase(self.status);
        let mut response = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.status,
            reason,
            self.content_type,
            self.body.len()
        )
        .into_bytes();
        response.extend_from_slice(&self.body);
        response
    }
}

pub fn bind_localhost(port: u16) -> Result<TcpListener> {
    Ok(TcpListener::bind(("127.0.0.1", port))?)
}

pub fn listener_url(listener: &TcpListener) -> Result<String> {
    Ok(format!("http://{}", listener.local_addr()?))
}

pub fn serve_forever(pack: Pack, listener: TcpListener) -> Result<()> {
    for stream in listener.incoming() {
        let stream = stream?;
        serve_stream(&pack, stream)?;
    }
    Ok(())
}

pub fn serve_once(pack: &Pack, listener: &TcpListener) -> Result<()> {
    let (stream, _) = listener.accept()?;
    serve_stream(pack, stream)
}

fn serve_stream(pack: &Pack, mut stream: TcpStream) -> Result<()> {
    let request = read_http_request(&mut stream)?;
    let response = handle_request(pack, &request)?.to_http_bytes();
    stream.write_all(&response)?;
    stream.flush()?;
    Ok(())
}

fn read_http_request(stream: &mut TcpStream) -> Result<String> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut buf = [0u8; 1024];
    let mut request = Vec::new();
    loop {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }
        request.extend_from_slice(&buf[..n]);
        if request.windows(4).any(|w| {
            w == b"

"
        }) || request.len() > 1024 * 1024
        {
            break;
        }
    }
    Ok(String::from_utf8(request)?)
}

pub fn handle_request(pack: &Pack, raw_request: &str) -> Result<HttpResponse> {
    let request_line = raw_request
        .lines()
        .next()
        .ok_or_else(|| anyhow!("empty HTTP request"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or_else(|| anyhow!("missing HTTP method"))?;
    let target = parts.next().ok_or_else(|| anyhow!("missing HTTP target"))?;
    if method != "GET" {
        return Ok(json_error(405, "method not allowed"));
    }
    route(pack, target)
}

fn route(pack: &Pack, target: &str) -> Result<HttpResponse> {
    let (path, query) = split_target(target);
    let query = match parse_query(query) {
        Ok(query) => query,
        Err(err) => return Ok(json_error(400, err.to_string())),
    };
    match path.as_str() {
        "/" => Ok(text_response(
            200,
            "text/html; charset=utf-8",
            viewer::index_html(),
        )),
        "/app.js" => Ok(text_response(
            200,
            "application/javascript; charset=utf-8",
            viewer::app_js(),
        )),
        "/style.css" => Ok(text_response(
            200,
            "text/css; charset=utf-8",
            viewer::style_css(),
        )),
        "/api/search" => {
            let Ok(q) = required_query(&query, "q") else {
                return Ok(json_error(400, "missing query parameter: q"));
            };
            json_response(api::search_with_filters(
                pack,
                q,
                api::SearchFilters {
                    note_type: query.get("type").map(String::as_str),
                    tag: query.get("tag").map(String::as_str),
                    from: query.get("from").map(String::as_str),
                    to: query.get("to").map(String::as_str),
                    k: read_usize(&query, "k", 10),
                },
            )?)
        }
        "/api/ask" => {
            let Ok(q) = required_query(&query, "q") else {
                return Ok(json_error(400, "missing query parameter: q"));
            };
            json_response(api::ask(pack, q, read_usize(&query, "k", 5))?)
        }
        "/api/facets" => json_response(api::facets(pack)?),
        "/api/gallery" => json_response(api::gallery(
            pack,
            query.get("type").map(String::as_str),
            read_usize(&query, "k", 20),
        )?),
        "/api/timeline" => json_response(api::timeline(
            pack,
            query.get("from").map(String::as_str),
            query.get("to").map(String::as_str),
            query.get("type").map(String::as_str),
            read_usize(&query, "k", 20),
        )?),
        "/api/graph" => json_response(api::graph(
            pack,
            query.get("type").map(String::as_str),
            read_usize(&query, "limit", 100),
        )?),
        _ => {
            if let Some(id) = path.strip_prefix("/api/notes/") {
                return api_result(api::note(pack, &percent_decode(id)?));
            }
            if let Some(id) = path.strip_prefix("/api/related/") {
                return api_result(api::related(
                    pack,
                    &percent_decode(id)?,
                    read_usize(&query, "depth", 1),
                ));
            }
            Ok(json_error(404, "not found"))
        }
    }
}

fn api_result<T: Serialize>(result: Result<T>) -> Result<HttpResponse> {
    match result {
        Ok(value) => json_response(value),
        Err(err) if err.to_string().contains("not found") => Ok(json_error(404, err.to_string())),
        Err(err) => Ok(json_error(500, err.to_string())),
    }
}

fn text_response(status: u16, content_type: &'static str, body: &'static str) -> HttpResponse {
    HttpResponse {
        status,
        content_type,
        body: body.as_bytes().to_vec(),
    }
}

fn json_response<T: Serialize>(value: T) -> Result<HttpResponse> {
    Ok(HttpResponse {
        status: 200,
        content_type: "application/json; charset=utf-8",
        body: serde_json::to_vec(&value)?,
    })
}

fn json_error(status: u16, message: impl Into<String>) -> HttpResponse {
    HttpResponse {
        status,
        content_type: "application/json; charset=utf-8",
        body: serde_json::to_vec(&json!({ "error": message.into() })).unwrap_or_default(),
    }
}

fn split_target(target: &str) -> (String, &str) {
    match target.split_once('?') {
        Some((path, query)) => (path.to_string(), query),
        None => (target.to_string(), ""),
    }
}

fn parse_query(raw: &str) -> Result<HashMap<String, String>> {
    let mut out = HashMap::new();
    if raw.is_empty() {
        return Ok(out);
    }
    for pair in raw.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        out.insert(percent_decode(key)?, percent_decode(value)?);
    }
    Ok(out)
}

fn required_query<'a>(query: &'a HashMap<String, String>, key: &str) -> Result<&'a str> {
    query
        .get(key)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("missing query parameter: {key}"))
}

fn read_usize(query: &HashMap<String, String>, key: &str, default: usize) -> usize {
    query
        .get(key)
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn percent_decode(input: &str) -> Result<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' => {
                if i + 2 >= bytes.len() {
                    bail!("invalid percent escape in URL");
                }
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3])?;
                out.push(u8::from_str_radix(hex, 16)?);
                i += 3;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    Ok(String::from_utf8(out)?)
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        _ => "OK",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pack_core::pack::Pack;
    use tempfile::tempdir;

    #[test]
    fn api_search_http_returns_json() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(root.join("notes/hook.md"), "---\ntitle: 훅\n---\n클릭 훅").unwrap();
        let pack = Pack::open(&root).unwrap();
        pack.build_index().unwrap();

        let response = handle_request(
            &pack,
            "GET /api/search?q=%ED%9B%85&k=5 HTTP/1.1\r\nHost: localhost\r\n\r\n",
        )
        .unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "application/json; charset=utf-8");
        let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(body["hits"][0]["note_id"], "hook");
    }

    #[test]
    fn api_search_missing_query_returns_400_json() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        let pack = Pack::open(&root).unwrap();

        let response =
            handle_request(&pack, "GET /api/search HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        assert_eq!(response.status, 400);
        assert_eq!(response.content_type, "application/json; charset=utf-8");
        let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert!(body["error"]
            .as_str()
            .unwrap()
            .contains("missing query parameter: q"));
    }

    #[test]
    fn api_note_http_returns_404_for_missing_note() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        let pack = Pack::open(&root).unwrap();

        let response = handle_request(
            &pack,
            "GET /api/notes/missing HTTP/1.1\r\nHost: localhost\r\n\r\n",
        )
        .unwrap();
        assert_eq!(response.status, 404);
        let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert!(body["error"].as_str().unwrap().contains("note not found"));
    }

    #[test]
    fn serves_static_viewer_shell() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        let pack = Pack::open(&root).unwrap();

        let response = handle_request(
            &pack,
            "GET / HTTP/1.1
Host: localhost

",
        )
        .unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "text/html; charset=utf-8");
        let html = String::from_utf8(response.body).unwrap();
        assert!(html.contains("ontopack"));
        assert!(html.contains("/app.js"));
        assert!(html.contains("ask-form"));
        assert!(html.contains("type-filter"));
        assert!(html.contains("gallery"));
    }

    #[test]
    fn viewer_js_reruns_search_when_filters_change() {
        let js = viewer::app_js();
        assert!(js.contains("async function refreshForFilters()"));
        assert!(js.contains("q ? search(q) : Promise.resolve()"));
        assert!(js.contains("addEventListener('change', refreshForFilters)"));
    }

    #[test]
    fn api_ask_http_returns_context_blocks() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(root.join("notes/hook.md"), "---\ntitle: 훅\n---\n클릭 훅").unwrap();
        let pack = Pack::open(&root).unwrap();
        pack.build_index().unwrap();

        let response = handle_request(
            &pack,
            "GET /api/ask?q=%ED%9B%85&k=3 HTTP/1.1\r\nHost: localhost\r\n\r\n",
        )
        .unwrap();
        assert_eq!(response.status, 200);
        let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(body["answer_mode"], "external_llm_required");
        assert_eq!(body["context_blocks"][0]["note_id"], "hook");
    }

    #[test]
    fn api_gallery_http_returns_asset_cards() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("p");
        Pack::init(&root, "p").unwrap();
        std::fs::write(
            root.join("notes/pic.md"),
            "---\ntype: image\ntitle: Pic\nasset: assets/pic.png\n---\n캡션",
        )
        .unwrap();
        let pack = Pack::open(&root).unwrap();

        let response = handle_request(
            &pack,
            "GET /api/gallery?k=5 HTTP/1.1\r\nHost: localhost\r\n\r\n",
        )
        .unwrap();
        assert_eq!(response.status, 200);
        let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(body["items"][0]["id"], "pic");
        assert_eq!(body["items"][0]["asset"], "assets/pic.png");
    }
}
