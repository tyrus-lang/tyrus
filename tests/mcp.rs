use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::{Duration, timeout};

#[tokio::test]
async fn stdio_server_lists_and_calls_documentation_tools() {
    let directory = tempfile::tempdir().unwrap();
    let generation = directory.path().join("cache").join("fixture");
    std::fs::create_dir_all(&generation).unwrap();
    let index =
        "# Tyrus Documentation v0.1.0\n\n- [Guide](https://tyruslang.org/guide) (guide.md)\n";
    let full = "# Tyrus Full Documentation v0.1.0\n\n# File: guide.md\n\n# Guide\n\nMCP documentation search fixture.\n";
    std::fs::write(generation.join("llms.txt"), index).unwrap();
    std::fs::write(generation.join("llms-full.txt"), full).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(index.as_bytes());
    hasher.update([0]);
    hasher.update(full.as_bytes());
    let hash = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    std::fs::write(
        generation.join("metadata.json"),
        serde_json::to_vec(&json!({
            "compilerVersion": "0.1.0",
            "docsVersion": "v0.1.0",
            "updatedAt": "2026-01-01T00:00:00Z",
            "contentSha256": hash,
        }))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        directory.path().join("current.json"),
        br#"{"generation":"fixture"}"#,
    )
    .unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .args(["mcp", "serve"])
        .env("TYRUS_MCP_HOME", directory.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    send(&mut input, json!({
        "jsonrpc":"2.0", "id":1, "method":"initialize",
        "params":{"protocolVersion":"2025-11-25", "capabilities":{}, "clientInfo":{"name":"test","version":"1"}}
    })).await;
    let initialized = read(&mut output).await;
    assert_eq!(initialized["id"], 1);
    assert!(initialized["result"].is_object());

    input
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
        .await
        .unwrap();
    send(
        &mut input,
        json!({"jsonrpc":"2.0", "id":2, "method":"tools/list", "params":{}}),
    )
    .await;
    let listed = read(&mut output).await;
    let tools = listed["result"]["tools"].as_array().unwrap();
    assert!(tools.iter().any(|tool| tool["name"] == "search_docs"));
    assert!(tools.iter().any(|tool| tool["name"] == "check_source"));
    send(
        &mut input,
        json!({ "jsonrpc":"2.0", "id":3, "method":"resources/list", "params":{} }),
    )
    .await;
    let resources = read(&mut output).await;
    assert!(
        resources["result"]["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|resource| { resource["uri"] == "tyrus://docs/guide.md" })
    );
    send(
        &mut input,
        json!({ "jsonrpc":"2.0", "id":4, "method":"prompts/list", "params":{} }),
    )
    .await;
    let prompts = read(&mut output).await;
    assert!(
        prompts["result"]["prompts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|prompt| { prompt["name"] == "generate_tyrus_code" })
    );

    send(
        &mut input,
        json!({
            "jsonrpc":"2.0", "id":5, "method":"tools/call",
            "params":{"name":"search_docs", "arguments":{"query":"documentation search"}}
        }),
    )
    .await;
    let searched = read(&mut output).await;
    assert_eq!(searched["id"], 5);
    let result = &searched["result"];
    assert!(result.to_string().contains("guide.md"));
    assert!(
        result
            .to_string()
            .contains("MCP documentation search fixture")
    );

    send(
        &mut input,
        json!({
            "jsonrpc":"2.0", "id":6, "method":"tools/call",
            "params":{"name":"check_source", "arguments":{"code":"const answer: i32 = 42;"}}
        }),
    )
    .await;
    let checked = read(&mut output).await;
    assert!(checked["result"].to_string().contains("\"ok\":true"));

    send(
        &mut input,
        json!({
            "jsonrpc":"2.0", "id":7, "method":"tools/call",
            "params":{"name":"get_page", "arguments":{"path":"../secret.md"}}
        }),
    )
    .await;
    let rejected = read(&mut output).await;
    assert!(rejected["result"].to_string().contains("page_not_found"));

    send(
        &mut input,
        json!({
            "jsonrpc":"2.0", "id":8, "method":"tools/call",
            "params":{"name":"check_source", "arguments":{"code":"x".repeat(128 * 1024 + 1)}}
        }),
    )
    .await;
    let oversized = read(&mut output).await;
    assert!(oversized["result"].to_string().contains("source_too_large"));

    drop(input);
    let status = timeout(Duration::from_secs(3), child.wait())
        .await
        .unwrap()
        .unwrap();
    assert!(status.success());
}

async fn send(input: &mut tokio::process::ChildStdin, message: Value) {
    input
        .write_all(serde_json::to_string(&message).unwrap().as_bytes())
        .await
        .unwrap();
    input.write_all(b"\n").await.unwrap();
    input.flush().await.unwrap();
}

async fn read(output: &mut BufReader<tokio::process::ChildStdout>) -> Value {
    let mut line = String::new();
    timeout(Duration::from_secs(5), output.read_line(&mut line))
        .await
        .unwrap()
        .unwrap();
    serde_json::from_str(line.trim()).unwrap()
}
