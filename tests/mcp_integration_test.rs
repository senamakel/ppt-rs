//! MCP server integration tests
//!
//! Tests the ppt_mcp binary end-to-end over stdio JSON-RPC transport.

use std::io::{BufRead, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serial_test::serial;

/// Keeps the MCP subprocess alive until the test ends, then kills it (avoids zombies / races).
struct ChildGuard(Option<Child>);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn start_server() -> (ChildGuard, ChildStdin, std::io::BufReader<ChildStdout>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ppt_mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start ppt_mcp");

    let stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let reader = std::io::BufReader::new(stdout);

    (ChildGuard(Some(child)), stdin, reader)
}

fn send(stdin: &mut ChildStdin, id: u64, method: &str, params: serde_json::Value) {
    let msg = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    });
    writeln!(stdin, "{}", serde_json::to_string(&msg).unwrap()).unwrap();
    stdin.flush().unwrap();
}

fn recv(reader: &mut std::io::BufReader<ChildStdout>) -> serde_json::Value {
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .expect("Failed to read response");
    let trimmed = line.trim();
    assert!(
        !trimmed.is_empty(),
        "empty MCP stdout line (ppt_mcp may have exited early); build with `cargo test --features mcp --test mcp_integration_test`"
    );
    serde_json::from_str(trimmed).expect("Failed to parse JSON response")
}

/// rmcp expects a spec-compliant `initialize` payload (see MCP handshake).
fn init_params() -> serde_json::Value {
    serde_json::json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {
            "name": "ppt-rs-integration-test",
            "version": "1.0.0"
        }
    })
}

fn notify_initialized(stdin: &mut ChildStdin) {
    let msg = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    });
    writeln!(stdin, "{}", serde_json::to_string(&msg).unwrap()).unwrap();
    stdin.flush().unwrap();
}

fn handshake(stdin: &mut ChildStdin, reader: &mut std::io::BufReader<ChildStdout>) {
    send(stdin, 1, "initialize", init_params());
    let _ = recv(reader);
    notify_initialized(stdin);
}

fn tmp_path(name: &str) -> String {
    let dir = std::env::temp_dir().join("ppt_rs_mcp_integration");
    let _ = std::fs::create_dir_all(&dir);
    dir.join(format!("ppt_mcp_test_{name}"))
        .to_string_lossy()
        .into_owned()
}

fn cleanup(path: &str) {
    let _ = std::fs::remove_file(path);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn test_initialize() {
    let (_guard, mut stdin, mut reader) = start_server();
    send(&mut stdin, 1, "initialize", init_params());
    let resp = recv(&mut reader);

    assert_eq!(resp["id"], 1);
    assert_eq!(resp["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(resp["result"]["serverInfo"]["name"], "ppt-rs");
    assert!(resp["result"]["capabilities"]["tools"].is_object());
}

#[test]
#[serial]
fn test_tools_list() {
    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    send(&mut stdin, 2, "tools/list", serde_json::json!({}));
    let resp = recv(&mut reader);

    assert_eq!(resp["id"], 2);
    let tools = resp["result"]["tools"].as_array().unwrap();
    assert!(tools.len() >= 12);

    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"create_presentation"));
    assert!(names.contains(&"markdown_to_pptx"));
    assert!(names.contains(&"get_pptx_info"));
    assert!(names.contains(&"export_pptx"));
    assert!(names.contains(&"merge_pptx"));
    assert!(names.contains(&"validate_pptx"));
    assert!(names.contains(&"create_presentation_with_tables"));
    assert!(names.contains(&"create_presentation_with_charts"));
    assert!(names.contains(&"compress_pptx"));
    assert!(names.contains(&"repair_pptx"));
    assert!(names.contains(&"apply_theme"));
    assert!(names.contains(&"diff_pptx"));

    for tool in tools {
        assert!(tool["name"].is_string());
        assert!(tool["description"].is_string());
        assert!(tool["inputSchema"]["type"] == "object");
        assert!(tool["inputSchema"]["properties"].is_object());
    }
}

#[test]
#[serial]
fn test_method_not_found() {
    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);
    send(&mut stdin, 2, "nonexistent_method", serde_json::json!({}));
    let resp = recv(&mut reader);

    assert_eq!(resp["id"], 2);
    assert_eq!(resp["error"]["code"], -32601);
}

#[test]
#[serial]
fn test_create_presentation() {
    let path = tmp_path("create.pptx");
    cleanup(&path);

    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "create_presentation",
            "arguments": {
                "title": "MCP Test",
                "output_path": &path,
                "slides": [
                    { "title": "Slide 1", "bullets": ["Hello", "World"] },
                    { "title": "Slide 2", "bullets": ["Alpha", "Beta"] },
                ]
            }
        }),
    );
    let resp = recv(&mut reader);

    assert_eq!(resp["id"], 2);
    assert!(!resp["result"]["isError"].as_bool().unwrap_or(false));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Created presentation"));
    assert!(text.contains("2 slide(s)"));
    assert!(std::path::Path::new(&path).exists());

    cleanup(&path);
}

#[test]
#[serial]
fn test_create_presentation_empty_slides_error() {
    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "create_presentation",
            "arguments": {
                "title": "Empty",
                "output_path": "/tmp/never.pptx",
                "slides": []
            }
        }),
    );
    let resp = recv(&mut reader);

    assert_eq!(resp["id"], 2);
    assert!(resp["result"]["isError"].as_bool().unwrap_or(false));
    assert!(
        resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("No slides")
    );
}

#[test]
#[serial]
fn test_markdown_to_pptx() {
    let path = tmp_path("md.pptx");
    cleanup(&path);

    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "markdown_to_pptx",
            "arguments": {
                "markdown": "# Intro\n- Point 1\n- Point 2\n\n# Summary\n- Done\n",
                "output_path": &path,
                "title": "MD Test"
            }
        }),
    );
    let resp = recv(&mut reader);

    assert_eq!(resp["id"], 2);
    assert!(!resp["result"]["isError"].as_bool().unwrap_or(false));
    assert!(
        resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("2 slide(s)")
    );
    assert!(std::path::Path::new(&path).exists());

    cleanup(&path);
}

#[test]
#[serial]
fn test_markdown_to_pptx_missing_param() {
    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "markdown_to_pptx",
            "arguments": {
                "output_path": "/tmp/never.pptx"
            }
        }),
    );
    let resp = recv(&mut reader);

    // rmcp 2.2.0 returns missing-param as a tool-level error (isError),
    // not a JSON-RPC -32602 error.
    assert!(
        resp["result"]["isError"].as_bool().unwrap_or(false),
        "expected isError for missing param; got: {resp}"
    );
}

#[test]
#[serial]
fn test_get_pptx_info() {
    let path = tmp_path("info.pptx");
    cleanup(&path);

    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    // First create a file
    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "create_presentation",
            "arguments": {
                "title": "Info Test",
                "output_path": &path,
                "slides": [
                    { "title": "First Slide", "bullets": ["A", "B"] },
                    { "title": "Second Slide", "bullets": ["C"] },
                ]
            }
        }),
    );
    let _ = recv(&mut reader);

    // Now get info
    send(
        &mut stdin,
        3,
        "tools/call",
        serde_json::json!({
            "name": "get_pptx_info",
            "arguments": { "path": &path }
        }),
    );
    let resp = recv(&mut reader);

    assert_eq!(resp["id"], 3);
    assert!(!resp["result"]["isError"].as_bool().unwrap_or(false));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let info: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(info["slide_count"], 2);
    assert_eq!(info["slides"][0]["title"], "First Slide");
    assert_eq!(info["slides"][0]["bullet_count"], 2);
    assert_eq!(info["slides"][1]["title"], "Second Slide");
    assert_eq!(info["slides"][1]["bullet_count"], 1);

    cleanup(&path);
}

#[test]
#[serial]
fn test_get_pptx_info_file_not_found() {
    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "get_pptx_info",
            "arguments": { "path": "/nonexistent/file.pptx" }
        }),
    );
    let resp = recv(&mut reader);

    assert!(resp["result"]["isError"].as_bool().unwrap_or(false));
    assert!(
        resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Error")
    );
}

#[test]
#[serial]
fn test_export_pptx_html() {
    let pptx_path = tmp_path("export_src.pptx");
    let html_path = tmp_path("export_out.html");
    cleanup(&pptx_path);
    cleanup(&html_path);

    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    // Create a file first
    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "create_presentation",
            "arguments": {
                "title": "Export Test",
                "output_path": &pptx_path,
                "slides": [{ "title": "Slide 1", "bullets": ["X"] }]
            }
        }),
    );
    let _ = recv(&mut reader);

    // Export to HTML
    send(
        &mut stdin,
        3,
        "tools/call",
        serde_json::json!({
            "name": "export_pptx",
            "arguments": {
                "input_path": &pptx_path,
                "output_path": &html_path,
                "format": "html"
            }
        }),
    );
    let resp = recv(&mut reader);

    assert!(!resp["result"]["isError"].as_bool().unwrap_or(false));
    assert!(std::path::Path::new(&html_path).exists());

    cleanup(&pptx_path);
    cleanup(&html_path);
}

#[test]
#[serial]
fn test_export_pptx_markdown() {
    let pptx_path = tmp_path("export_md_src.pptx");
    let md_path = tmp_path("export_md_out.md");
    cleanup(&pptx_path);
    cleanup(&md_path);

    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "create_presentation",
            "arguments": {
                "title": "MD Export",
                "output_path": &pptx_path,
                "slides": [{ "title": "Slide 1", "bullets": ["Bullet 1"] }]
            }
        }),
    );
    let _ = recv(&mut reader);

    send(
        &mut stdin,
        3,
        "tools/call",
        serde_json::json!({
            "name": "export_pptx",
            "arguments": {
                "input_path": &pptx_path,
                "output_path": &md_path,
                "format": "markdown"
            }
        }),
    );
    let resp = recv(&mut reader);

    assert!(!resp["result"]["isError"].as_bool().unwrap_or(false));
    assert!(std::path::Path::new(&md_path).exists());

    let content = std::fs::read_to_string(&md_path).unwrap();
    assert!(content.contains("Slide 1"));

    cleanup(&pptx_path);
    cleanup(&md_path);
}

#[test]
#[serial]
fn test_export_pptx_unsupported_format() {
    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "export_pptx",
            "arguments": {
                "input_path": "/tmp/x.pptx",
                "output_path": "/tmp/x.docx",
                "format": "docx"
            }
        }),
    );
    let resp = recv(&mut reader);

    assert!(resp["result"]["isError"].as_bool().unwrap_or(false));
    assert!(
        resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Unsupported format")
    );
}

#[test]
#[serial]
fn test_validate_pptx() {
    let path = tmp_path("validate.pptx");
    cleanup(&path);

    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    // Create a valid file first
    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "create_presentation",
            "arguments": {
                "title": "Validate Test",
                "output_path": &path,
                "slides": [{ "title": "Slide", "bullets": ["X"] }]
            }
        }),
    );
    let _ = recv(&mut reader);

    // Validate it
    send(
        &mut stdin,
        3,
        "tools/call",
        serde_json::json!({
            "name": "validate_pptx",
            "arguments": { "path": &path }
        }),
    );
    let resp = recv(&mut reader);

    assert!(!resp["result"]["isError"].as_bool().unwrap_or(false));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Validation passed") || text.contains("issue(s)"));

    cleanup(&path);
}

#[test]
#[serial]
fn test_validate_pptx_invalid_file() {
    let path = tmp_path("invalid.pptx");
    cleanup(&path);
    std::fs::write(&path, "not a pptx file").unwrap();

    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "validate_pptx",
            "arguments": { "path": &path }
        }),
    );
    let resp = recv(&mut reader);

    // validate_pptx returns issues as a success result (the tool ran
    // successfully, it just found problems). An invalid ZIP should produce
    // issue text mentioning the ZIP error.
    assert!(!resp["result"]["isError"].as_bool().unwrap_or(false));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("issue(s)"),
        "expected issue text; got: {text}"
    );

    cleanup(&path);
}

#[test]
#[serial]
fn test_merge_pptx() {
    let path_a = tmp_path("merge_a.pptx");
    let path_b = tmp_path("merge_b.pptx");
    let merged = tmp_path("merged.pptx");
    cleanup(&path_a);
    cleanup(&path_b);
    cleanup(&merged);

    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    // Create first file
    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "create_presentation",
            "arguments": {
                "title": "Part A",
                "output_path": &path_a,
                "slides": [{ "title": "A1", "bullets": ["a1"] }]
            }
        }),
    );
    let _ = recv(&mut reader);

    // Create second file
    send(
        &mut stdin,
        3,
        "tools/call",
        serde_json::json!({
            "name": "create_presentation",
            "arguments": {
                "title": "Part B",
                "output_path": &path_b,
                "slides": [
                    { "title": "B1", "bullets": ["b1"] },
                    { "title": "B2", "bullets": ["b2"] },
                ]
            }
        }),
    );
    let _ = recv(&mut reader);

    // Merge
    send(
        &mut stdin,
        4,
        "tools/call",
        serde_json::json!({
            "name": "merge_pptx",
            "arguments": {
                "input_paths": [&path_a, &path_b],
                "output_path": &merged
            }
        }),
    );
    let resp = recv(&mut reader);

    assert!(!resp["result"]["isError"].as_bool().unwrap_or(false));
    assert!(
        resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("3 total slides")
    );

    // Verify merged file
    send(
        &mut stdin,
        5,
        "tools/call",
        serde_json::json!({
            "name": "get_pptx_info",
            "arguments": { "path": &merged }
        }),
    );
    let resp = recv(&mut reader);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let info: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(info["slide_count"], 3);

    cleanup(&path_a);
    cleanup(&path_b);
    cleanup(&merged);
}

#[test]
#[serial]
fn test_merge_pptx_single_file_error() {
    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "merge_pptx",
            "arguments": {
                "input_paths": ["/tmp/only_one.pptx"],
                "output_path": "/tmp/out.pptx"
            }
        }),
    );
    let resp = recv(&mut reader);

    assert!(resp["result"]["isError"].as_bool().unwrap_or(false));
    assert!(
        resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("At least 2")
    );
}

#[test]
#[serial]
fn test_create_presentation_with_tables() {
    let path = tmp_path("tables.pptx");
    cleanup(&path);

    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "create_presentation_with_tables",
            "arguments": {
                "title": "Table Test",
                "output_path": &path,
                "tables": [
                    {
                        "slide_title": "Team Directory",
                        "rows": [
                            ["Name", "Role", "Status"],
                            ["Alice", "Engineer", "Active"],
                            ["Bob", "Designer", "Active"]
                        ]
                    }
                ]
            }
        }),
    );
    let resp = recv(&mut reader);

    assert!(!resp["result"]["isError"].as_bool().unwrap_or(false));
    assert!(
        resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("1 table slide")
    );
    assert!(std::path::Path::new(&path).exists());

    // Verify the table was created via get_pptx_info
    send(
        &mut stdin,
        3,
        "tools/call",
        serde_json::json!({
            "name": "get_pptx_info",
            "arguments": { "path": &path }
        }),
    );
    let resp = recv(&mut reader);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let info: serde_json::Value = serde_json::from_str(text).unwrap();
    assert!(
        info["slides"][0]["features"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("table"))
    );

    cleanup(&path);
}

#[test]
#[serial]
fn test_create_presentation_with_charts() {
    let path = tmp_path("charts.pptx");
    cleanup(&path);

    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "create_presentation_with_charts",
            "arguments": {
                "title": "Chart Test",
                "output_path": &path,
                "charts": [
                    {
                        "slide_title": "Sales Data",
                        "chart_title": "Q1 Sales",
                        "chart_type": "bar",
                        "categories": ["Jan", "Feb", "Mar"],
                        "series": [
                            { "name": "2024", "values": [100.0, 150.0, 120.0] },
                            { "name": "2025", "values": [120.0, 180.0, 150.0] }
                        ]
                    },
                    {
                        "slide_title": "Market Share",
                        "chart_title": "Pie Chart",
                        "chart_type": "pie",
                        "categories": ["A", "B", "C"],
                        "series": [
                            { "name": "Share", "values": [50.0, 30.0, 20.0] }
                        ]
                    }
                ]
            }
        }),
    );
    let resp = recv(&mut reader);

    assert!(!resp["result"]["isError"].as_bool().unwrap_or(false));
    assert!(
        resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("2 chart slide")
    );
    assert!(std::path::Path::new(&path).exists());

    cleanup(&path);
}

#[test]
#[serial]
fn test_unknown_tool() {
    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "nonexistent_tool",
            "arguments": {}
        }),
    );
    let resp = recv(&mut reader);

    assert_eq!(resp["error"]["code"], -32602);
    assert!(
        resp["error"]["message"]
            .as_str()
            .unwrap()
            .to_ascii_lowercase()
            .contains("tool")
    );
}

#[test]
#[serial]
fn test_initialize_requires_full_params() {
    let (_guard, mut stdin, mut reader) = start_server();
    send(&mut stdin, 1, "initialize", serde_json::json!({}));

    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(
        line.trim().is_empty(),
        "initialize with empty params must fail handshake (rmcp); got: {:?}",
        line.trim()
    );
}

#[test]
#[serial]
fn test_ping() {
    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);
    send(&mut stdin, 2, "ping", serde_json::json!({}));
    let resp = recv(&mut reader);

    assert_eq!(resp["id"], 2);
    assert!(resp["result"].is_object());
}

// ---------------------------------------------------------------------------
// compress_pptx / repair_pptx / apply_theme
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn test_compress_pptx() {
    let src = tmp_path("compress_src.pptx");
    let out = tmp_path("compress_out.pptx");
    cleanup(&src);
    cleanup(&out);

    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    // Create a source file first
    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "create_presentation",
            "arguments": {
                "title": "Compress Me",
                "output_path": &src,
                "slides": [{ "title": "S1", "bullets": ["a", "b"] }]
            }
        }),
    );
    let _ = recv(&mut reader);

    // Compress it
    send(
        &mut stdin,
        3,
        "tools/call",
        serde_json::json!({
            "name": "compress_pptx",
            "arguments": {
                "input_path": &src,
                "output_path": &out,
                "level": "medium"
            }
        }),
    );
    let resp = recv(&mut reader);

    assert!(!resp["result"]["isError"].as_bool().unwrap_or(false));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Compressed"));
    assert!(text.contains("reduction"));
    assert!(std::path::Path::new(&out).exists());

    cleanup(&src);
    cleanup(&out);
}

#[test]
#[serial]
fn test_compress_pptx_bad_level() {
    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "compress_pptx",
            "arguments": {
                "input_path": "/tmp/x.pptx",
                "output_path": "/tmp/y.pptx",
                "level": "ultra"
            }
        }),
    );
    let resp = recv(&mut reader);
    assert!(resp["result"]["isError"].as_bool().unwrap_or(false));
    assert!(
        resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Unsupported level")
    );
}

#[test]
#[serial]
fn test_repair_pptx() {
    let src = tmp_path("repair_src.pptx");
    let out = tmp_path("repair_out.pptx");
    cleanup(&src);
    cleanup(&out);

    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    // Create a valid source file
    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "create_presentation",
            "arguments": {
                "title": "Repair Me",
                "output_path": &src,
                "slides": [{ "title": "S1", "bullets": ["x"] }]
            }
        }),
    );
    let _ = recv(&mut reader);

    // Repair it (should find no issues but still save a copy)
    send(
        &mut stdin,
        3,
        "tools/call",
        serde_json::json!({
            "name": "repair_pptx",
            "arguments": {
                "input_path": &src,
                "output_path": &out
            }
        }),
    );
    let resp = recv(&mut reader);

    assert!(!resp["result"]["isError"].as_bool().unwrap_or(false));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("No issues found") || text.contains("issue(s)"));
    assert!(std::path::Path::new(&out).exists());

    cleanup(&src);
    cleanup(&out);
}

#[test]
#[serial]
fn test_apply_theme() {
    let src = tmp_path("theme_src.pptx");
    let out = tmp_path("theme_out.pptx");
    cleanup(&src);
    cleanup(&out);

    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    // Create a source file
    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "create_presentation",
            "arguments": {
                "title": "Theme Me",
                "output_path": &src,
                "slides": [{ "title": "S1", "bullets": ["a"] }]
            }
        }),
    );
    let _ = recv(&mut reader);

    // Apply a custom theme
    send(
        &mut stdin,
        3,
        "tools/call",
        serde_json::json!({
            "name": "apply_theme",
            "arguments": {
                "input_path": &src,
                "output_path": &out,
                "theme": {
                    "name": "Corporate Blue",
                    "palette": {
                        "primary": "1F497D",
                        "secondary": "4F81BD",
                        "accent": "9BBB59",
                        "background": "FFFFFF",
                        "text": "000000",
                        "light": "EEECE1",
                        "dark": "1F497D"
                    },
                    "fonts": {
                        "major": "Calibri Light",
                        "minor": "Calibri"
                    }
                }
            }
        }),
    );
    let resp = recv(&mut reader);

    assert!(!resp["result"]["isError"].as_bool().unwrap_or(false));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Applied theme"));
    assert!(std::path::Path::new(&out).exists());

    cleanup(&src);
    cleanup(&out);
}

#[test]
#[serial]
fn test_apply_theme_default() {
    let src = tmp_path("theme_def_src.pptx");
    let out = tmp_path("theme_def_out.pptx");
    cleanup(&src);
    cleanup(&out);

    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    send(
        &mut stdin,
        2,
        "tools/call",
        serde_json::json!({
            "name": "create_presentation",
            "arguments": {
                "title": "Default Theme",
                "output_path": &src,
                "slides": [{ "title": "S1", "bullets": ["a"] }]
            }
        }),
    );
    let _ = recv(&mut reader);

    // Apply default Office theme (omit theme arg)
    send(
        &mut stdin,
        3,
        "tools/call",
        serde_json::json!({
            "name": "apply_theme",
            "arguments": {
                "input_path": &src,
                "output_path": &out
            }
        }),
    );
    let resp = recv(&mut reader);

    assert!(!resp["result"]["isError"].as_bool().unwrap_or(false));
    assert!(std::path::Path::new(&out).exists());

    cleanup(&src);
    cleanup(&out);
}

#[test]
#[serial]
fn test_diff_pptx_equal_and_different() {
    let old_path = tmp_path("diff_old.pptx");
    let same_path = tmp_path("diff_same.pptx");
    let new_path = tmp_path("diff_new.pptx");
    cleanup(&old_path);
    cleanup(&same_path);
    cleanup(&new_path);

    let (_guard, mut stdin, mut reader) = start_server();
    handshake(&mut stdin, &mut reader);

    let slide = serde_json::json!([{ "title": "Intro", "bullets": ["One"] }]);
    for (id, path) in [(2, &old_path), (3, &same_path)] {
        send(
            &mut stdin,
            id,
            "tools/call",
            serde_json::json!({
                "name": "create_presentation",
                "arguments": {
                    "title": "Diff Base",
                    "output_path": path,
                    "slides": slide
                }
            }),
        );
        let _ = recv(&mut reader);
    }

    send(
        &mut stdin,
        4,
        "tools/call",
        serde_json::json!({
            "name": "create_presentation",
            "arguments": {
                "title": "Diff Base",
                "output_path": &new_path,
                "slides": [{ "title": "Intro", "bullets": ["Two"] }]
            }
        }),
    );
    let _ = recv(&mut reader);

    send(
        &mut stdin,
        5,
        "tools/call",
        serde_json::json!({
            "name": "diff_pptx",
            "arguments": {
                "old_path": &old_path,
                "new_path": &same_path,
                "json": true
            }
        }),
    );
    let equal_resp = recv(&mut reader);
    assert!(!equal_resp["result"]["isError"].as_bool().unwrap_or(false));
    let equal_text = equal_resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(equal_text.contains("\"equal\": true"));

    send(
        &mut stdin,
        6,
        "tools/call",
        serde_json::json!({
            "name": "diff_pptx",
            "arguments": {
                "old_path": &old_path,
                "new_path": &new_path
            }
        }),
    );
    let diff_resp = recv(&mut reader);
    assert!(!diff_resp["result"]["isError"].as_bool().unwrap_or(false));
    let diff_text = diff_resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(diff_text.contains("modified") || diff_text.contains("~ Slide"));

    cleanup(&old_path);
    cleanup(&same_path);
    cleanup(&new_path);
}
