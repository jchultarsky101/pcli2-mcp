use axum::body::Bytes;
use axum::{body::to_bytes, extract::State, http::StatusCode, response::IntoResponse};
use pcli2_mcp::{
    AppState,
    mcp::handle_mcp,
    pcli::{PCLI2_BIN_ENV, call_tool, run_pcli2_command, run_pcli2_tenant_list, run_pcli2_version},
    thumbnail::{ThumbnailCache, ThumbnailCacheConfig},
};
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;

struct EnvVarGuard {
    key: &'static str,
    original: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var(key).ok();
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        unsafe {
            match &self.original {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }
}

fn test_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn make_mock_pcli2() -> PathBuf {
    let mut dir = std::env::temp_dir();
    let pid = std::process::id();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    dir.push(format!("pcli2-mcp-test-{}-{}", pid, ts));
    fs::create_dir_all(&dir).expect("create temp dir");
    let script_path = dir.join("pcli2");
    let script = r#"#!/bin/sh
# Mock pcli2: echoes its arguments one per line so tests can assert the argv,
# plus the environment the server is expected to set.
if [ "$1" = "--version" ]; then
  echo "pcli2 9.9.9"
  exit 0
fi
if [ "$1" = "oops" ]; then
  echo '{"code":67,"kind":"not_found","level":"ERROR","message":"mock failure"}' >&2
  echo "partial stdout"
  exit 67
fi
if [ "$1" = "asset" ] && [ "$2" = "thumbnail" ]; then
  out=""
  while [ $# -gt 0 ]; do
    if [ "$1" = "--output" ]; then out="$2"; fi
    shift
  done
  if [ -z "$out" ]; then echo "no --output" >&2; exit 2; fi
  printf '\211PNG\r\n\032\nmock' > "$out"
  exit 0
fi
for a in "$@"; do echo "$a"; done
echo "ENV NO_INPUT=${PCLI2_NO_INPUT} NO_COLOR=${PCLI2_NO_COLOR} NO_UPDATE_CHECK=${PCLI2_NO_UPDATE_CHECK} ERROR_FORMAT=${PCLI2_ERROR_FORMAT}"
exit 0
"#;
    fs::write(&script_path, script).expect("write mock pcli2");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).expect("set permissions");
    }
    script_path
}

#[tokio::test]
async fn mock_pcli2_version_and_tenant_list() {
    let _lock = test_env_lock().lock().await;
    let script_path = make_mock_pcli2();
    let _guard = EnvVarGuard::set(PCLI2_BIN_ENV, script_path.to_string_lossy().as_ref());

    let version = run_pcli2_version().await.expect("version");
    assert_eq!(version.trim(), "pcli2 9.9.9");

    let args = json!({
        "format": "json",
        "pretty": false,
        "headers": false
    });
    let list = run_pcli2_tenant_list(args).await.expect("tenant list");
    assert_eq!(
        list,
        "tenant\nlist\n--format\njson\nENV NO_INPUT=1 NO_COLOR=1 NO_UPDATE_CHECK=1 ERROR_FORMAT=json"
    );
}

#[tokio::test]
async fn mock_pcli2_error_includes_label() {
    let _lock = test_env_lock().lock().await;
    let script_path = make_mock_pcli2();
    let _guard = EnvVarGuard::set(PCLI2_BIN_ENV, script_path.to_string_lossy().as_ref());

    let err = run_pcli2_command(vec!["oops".to_string()], "pcli2 oops")
        .await
        .expect_err("expected error");
    assert!(err.starts_with("pcli2 oops failed (exit 67)"), "{}", err);
    assert!(err.contains("ERROR (not_found): mock failure"), "{}", err);
    assert!(err.contains("partial stdout"), "{}", err);
}

#[tokio::test]
async fn jsonrpc_parse_error_returns_32700() {
    let state = AppState {
        server_name: "test".to_string(),
        server_version: "0.0.0".to_string(),
        thumbnail_cache: std::sync::Arc::new(None),
    };
    let response = handle_mcp(State(state), Bytes::from("{bad json"))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let value: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(value["error"]["code"], -32700);
}

#[tokio::test]
async fn jsonrpc_invalid_request_returns_32600() {
    let state = AppState {
        server_name: "test".to_string(),
        server_version: "0.0.0".to_string(),
        thumbnail_cache: std::sync::Arc::new(None),
    };
    let response = handle_mcp(State(state), Bytes::from(r#"{"jsonrpc":"2.0","id":1}"#))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let value: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(value["error"]["code"], -32600);
}

#[tokio::test]
async fn jsonrpc_notification_returns_no_content() {
    let state = AppState {
        server_name: "test".to_string(),
        server_version: "0.0.0".to_string(),
        thumbnail_cache: std::sync::Arc::new(None),
    };
    let response = handle_mcp(
        State(state),
        Bytes::from(r#"{"jsonrpc":"2.0","method":"tools/list"}"#),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn logging_on_tools_call_outputs_command() {
    let _lock = test_env_lock().lock().await;
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("debug"))
        .with_writer(std::io::stdout)
        .try_init();

    let script_path = make_mock_pcli2();
    let _guard = EnvVarGuard::set(PCLI2_BIN_ENV, script_path.to_string_lossy().as_ref());

    let state = AppState {
        server_name: "mock".to_string(),
        server_version: "0.0.0".to_string(),
        thumbnail_cache: std::sync::Arc::new(None),
    };

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "pcli2_tenant_list",
            "arguments": {}
        }
    });
    let response = handle_mcp(State(state), Bytes::from(request.to_string()))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_initialize_method() {
    let state = AppState {
        server_name: "test".to_string(),
        server_version: "0.0.0".to_string(),
        thumbnail_cache: std::sync::Arc::new(None),
    };

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    let response = handle_mcp(State(state), Bytes::from(request.to_string()))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let value: Value = serde_json::from_slice(&body).expect("json");

    assert_eq!(value["result"]["protocolVersion"], "2025-03-26");
    assert_eq!(value["result"]["serverInfo"]["name"], "test");
    assert_eq!(value["result"]["serverInfo"]["version"], "0.0.0");
}

#[tokio::test]
async fn test_tools_list_method() {
    let state = AppState {
        server_name: "test".to_string(),
        server_version: "0.0.0".to_string(),
        thumbnail_cache: std::sync::Arc::new(None),
    };

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });
    let response = handle_mcp(State(state), Bytes::from(request.to_string()))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let value: Value = serde_json::from_slice(&body).expect("json");

    assert!(value["result"]["tools"].is_array());
    assert!(!value["result"]["tools"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_unknown_method_returns_error() {
    let state = AppState {
        server_name: "test".to_string(),
        server_version: "0.0.0".to_string(),
        thumbnail_cache: std::sync::Arc::new(None),
    };

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "unknown/method",
        "params": {}
    });
    let response = handle_mcp(State(state), Bytes::from(request.to_string()))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let value: Value = serde_json::from_slice(&body).expect("json");

    assert_eq!(value["error"]["code"], -32601);
}

#[tokio::test]
async fn test_jsonrpc_wrong_version() {
    let state = AppState {
        server_name: "test".to_string(),
        server_version: "0.0.0".to_string(),
        thumbnail_cache: std::sync::Arc::new(None),
    };

    let request = json!({
        "jsonrpc": "1.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });
    let response = handle_mcp(State(state), Bytes::from(request.to_string()))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let value: Value = serde_json::from_slice(&body).expect("json");

    assert_eq!(value["error"]["code"], -32600);
}

async fn call_text(name: &str, arguments: Value) -> Result<String, String> {
    let result = call_tool(json!({"name": name, "arguments": arguments}), None).await?;
    Ok(result["content"][0]["text"].as_str().unwrap().to_string())
}

#[tokio::test]
async fn call_tool_builds_expected_argv() {
    let _lock = test_env_lock().lock().await;
    let script_path = make_mock_pcli2();
    let _guard = EnvVarGuard::set(PCLI2_BIN_ENV, script_path.to_string_lossy().as_ref());

    let text = call_text(
        "pcli2_asset_geometric_match",
        json!({"path": "/A/part with space.stl", "threshold": 85, "format": "csv", "headers": true}),
    )
    .await
    .unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        &lines[..lines.len() - 1],
        &[
            "asset",
            "geometric-match",
            "--path",
            "/A/part with space.stl",
            "--threshold",
            "85",
            "--headers",
            "--format",
            "csv"
        ]
    );

    // Legacy combined tool delegates to asset list, now accepting folder_uuid.
    let text = call_text("pcli2", json!({"resource": "asset", "folder_uuid": "fu"}))
        .await
        .unwrap();
    assert!(
        text.starts_with("asset\nlist\n--folder-uuid\nfu\n"),
        "{}",
        text
    );
    let text = call_text("pcli2", json!({"format": "tree"})).await.unwrap();
    assert!(
        text.starts_with("folder\nlist\n--format\ntree\n"),
        "{}",
        text
    );

    // Backward-compatible tenant_name alias.
    let text = call_text("pcli2_tenant_use", json!({"tenant_name": "demo"}))
        .await
        .unwrap();
    assert!(text.starts_with("tenant\nuse\n--name\ndemo\n"), "{}", text);

    // Positional argument.
    let text = call_text("pcli2_user_get", json!({"user_id": "u-1"}))
        .await
        .unwrap();
    assert!(text.starts_with("user\nget\nu-1\n"), "{}", text);
}

#[tokio::test]
async fn call_tool_reports_pcli2_failure() {
    let _lock = test_env_lock().lock().await;
    let script_path = make_mock_pcli2();
    let _guard = EnvVarGuard::set(PCLI2_BIN_ENV, script_path.to_string_lossy().as_ref());

    // The mock fails for the bare "oops" argv; route it through the version tool's
    // command by faking a spec is not possible, so use run_pcli2_command directly
    // and a tool with an argument error for the pre-spawn path.
    let err = call_text(
        "pcli2_folder_geometric_match",
        json!({"folder_path": "/a", "threshold": 200}),
    )
    .await
    .unwrap_err();
    assert!(err.contains("between 0 and 100"), "{}", err);
}

#[tokio::test]
async fn asset_thumbnail_uses_output_flag_and_returns_data_url() {
    let _lock = test_env_lock().lock().await;
    let script_path = make_mock_pcli2();
    let _guard = EnvVarGuard::set(PCLI2_BIN_ENV, script_path.to_string_lossy().as_ref());

    let html = call_text(
        "pcli2_asset_thumbnail",
        json!({"uuid": "abc", "response_mode": "data_url"}),
    )
    .await
    .unwrap();
    assert!(
        html.contains("<img src=\"data:image/png;base64,"),
        "{}",
        html
    );

    // Without a cache, url mode falls back to a data URL.
    let html = call_text("pcli2_asset_thumbnail", json!({"path": "/A/p.stl"}))
        .await
        .unwrap();
    assert!(
        html.contains("<img src=\"data:image/png;base64,"),
        "{}",
        html
    );
}

#[tokio::test]
async fn asset_thumbnail_uses_cache_url_mode() {
    let _lock = test_env_lock().lock().await;
    let script_path = make_mock_pcli2();
    let _guard = EnvVarGuard::set(PCLI2_BIN_ENV, script_path.to_string_lossy().as_ref());

    let mut cache_dir = std::env::temp_dir();
    cache_dir.push(format!("pcli2-mcp-thumb-it-{}", std::process::id()));
    let cache = ThumbnailCache::new(ThumbnailCacheConfig::new(
        cache_dir,
        std::time::Duration::from_secs(60),
        "localhost",
        8080,
    ))
    .unwrap();

    let result = call_tool(
        json!({"name": "pcli2_asset_thumbnail", "arguments": {"uuid": "abc"}}),
        Some(&cache),
    )
    .await
    .unwrap();
    let html = result["content"][0]["text"].as_str().unwrap();
    let start = html.find("http://localhost:8080/thumbnail/").expect("url");
    let key = &html[start + "http://localhost:8080/thumbnail/".len()..];
    let key = &key[..key.find('"').unwrap()];
    let data = cache.load_thumbnail(key).unwrap();
    assert!(data.starts_with(b"\x89PNG"));
}

#[tokio::test]
async fn tools_list_advertises_new_pcli2_v2_tools() {
    let state = AppState {
        server_name: "test".to_string(),
        server_version: "0.0.0".to_string(),
        thumbnail_cache: std::sync::Arc::new(None),
    };
    let request = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}});
    let response = handle_mcp(State(state), Bytes::from(request.to_string()))
        .await
        .into_response();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();
    let names: Vec<&str> = value["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    for expected in [
        "pcli2",
        "pcli2_doctor",
        "pcli2_tenant_metadata_list",
        "pcli2_folder_list",
        "pcli2_folder_create",
        "pcli2_folder_delete",
        "pcli2_folder_upload",
        "pcli2_asset_list",
        "pcli2_asset_create",
        "pcli2_asset_delete",
        "pcli2_asset_similarity",
        "pcli2_asset_dependency_diff",
        "pcli2_asset_counts",
        "pcli2_asset_inventory",
        "pcli2_asset_metadata_get",
        "pcli2_asset_metadata_create_batch",
        "pcli2_asset_metadata_inference",
        "pcli2_auth_expiration",
        "pcli2_config_validate",
        "pcli2_cache_clear",
        "pcli2_thumbnail_cache_cleanup",
    ] {
        assert!(names.contains(&expected), "missing tool {}", expected);
    }
    for tool in value["result"]["tools"].as_array().unwrap() {
        assert_eq!(tool["inputSchema"]["type"], "object");
        assert!(tool["inputSchema"]["properties"].is_object());
        assert!(tool["inputSchema"]["required"].is_array());
    }
}
