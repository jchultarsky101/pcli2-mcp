//! Runs pcli2 on behalf of MCP tool calls.
//!
//! Tool schemas and argument mapping live in [`crate::tools`]; this module
//! handles process execution, output capture, and the few tools that need
//! server-side handling (thumbnails and the thumbnail cache).

use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use serde_json::{Value, json};
use std::{
    env, fs,
    path::PathBuf,
    process::Stdio,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::io::{AsyncRead, AsyncReadExt};
use tracing::info;

use crate::thumbnail::ThumbnailCache;
use crate::tools::{TOOLS, ToolSpec, build_args, find_tool, schema};

pub const PCLI2_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30 * 60);
pub const MAX_PCLI2_OUTPUT_BYTES: usize = 200 * 1024 * 1024;
pub const PCLI2_BIN_ENV: &str = "PCLI2_BIN";

/// Environment applied to every pcli2 child process so it behaves well when
/// driven by a server: never prompt, no ANSI colour, no update-check chatter,
/// and machine-readable errors on stderr.
pub const PCLI2_CHILD_ENV: &[(&str, &str)] = &[
    ("PCLI2_NO_INPUT", "1"),
    ("PCLI2_NO_COLOR", "1"),
    ("PCLI2_NO_UPDATE_CHECK", "1"),
    ("PCLI2_ERROR_FORMAT", "json"),
];

const TOOL_LEGACY_LIST: &str = "pcli2";
const TOOL_ASSET_THUMBNAIL: &str = "pcli2_asset_thumbnail";
const TOOL_THUMBNAIL_CACHE_CLEANUP: &str = "pcli2_thumbnail_cache_cleanup";

pub fn tool_list() -> Vec<Value> {
    TOOLS.iter().map(schema).collect()
}

pub async fn call_tool(
    params: Value,
    thumbnail_cache: Option<&ThumbnailCache>,
) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing tool name".to_string())?;
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    match name {
        TOOL_LEGACY_LIST => text_result(run_legacy_list(args).await?),
        TOOL_ASSET_THUMBNAIL => {
            let src = run_asset_thumbnail(args, thumbnail_cache).await?;
            // `src` is either an HTTP URL served by this server or a data URI.
            let html = format!(
                r#"<!DOCTYPE html>
<html>
<head><title>Asset Thumbnail</title></head>
<body>
<img src="{}" alt="Asset Thumbnail" style="max-width: 100%; height: auto;">
</body>
</html>"#,
                src
            );
            text_result(html)
        }
        TOOL_THUMBNAIL_CACHE_CLEANUP => {
            let Some(cache) = thumbnail_cache else {
                return text_result("Thumbnail cache is not available".to_string());
            };
            match cache.cleanup_expired() {
                Ok(count) => text_result(format!("Cleaned up {} expired thumbnail(s)", count)),
                Err(err) => Err(format!("Thumbnail cache cleanup failed: {}", err)),
            }
        }
        _ => {
            let spec = find_tool(name).ok_or_else(|| format!("Unknown tool '{}'", name))?;
            let args = normalize_args(spec, args);
            text_result(run_spec(spec, &args).await?)
        }
    }
}

fn text_result(text: String) -> Result<Value, String> {
    Ok(json!({
        "content": [{
            "type": "text",
            "text": text
        }]
    }))
}

/// Backward-compatible argument aliases.
fn normalize_args(spec: &ToolSpec, mut args: Value) -> Value {
    if spec.name == "pcli2_tenant_use"
        && let Some(obj) = args.as_object_mut()
        && !obj.contains_key("name")
        && let Some(alias) = obj.get("tenant_name").cloned()
    {
        obj.insert("name".to_string(), alias);
    }
    args
}

fn label_for(spec: &ToolSpec) -> String {
    if spec.command.is_empty() {
        "pcli2".to_string()
    } else {
        format!("pcli2 {}", spec.command.join(" "))
    }
}

async fn run_spec(spec: &ToolSpec, args: &Value) -> Result<String, String> {
    let argv = build_args(spec, args)?;
    run_pcli2_command(argv, &label_for(spec)).await
}

/// The original combined `pcli2` tool: `resource` picks folder or asset listing.
async fn run_legacy_list(args: Value) -> Result<String, String> {
    let resource = args
        .get("resource")
        .and_then(|v| v.as_str())
        .unwrap_or("folder");
    let delegate = match resource {
        "folder" => "pcli2_folder_list",
        "asset" => "pcli2_asset_list",
        other => {
            return Err(format!(
                "Invalid argument 'resource': '{}' is not one of folder, asset",
                other
            ));
        }
    };
    let spec = find_tool(delegate).expect("delegate tool exists");
    run_spec(spec, &args).await
}

async fn run_asset_thumbnail(
    args: Value,
    thumbnail_cache: Option<&ThumbnailCache>,
) -> Result<String, String> {
    let spec = find_tool(TOOL_ASSET_THUMBNAIL).expect("thumbnail tool exists");
    let mut argv = build_args(spec, &args)?;
    let temp_path = temp_thumbnail_path()?;
    let temp_path_str = temp_path
        .to_str()
        .ok_or_else(|| "Failed to build temporary thumbnail path".to_string())?;
    argv.push("--output".to_string());
    argv.push(temp_path_str.to_string());
    run_pcli2_command(argv, &label_for(spec)).await?;

    let bytes_result =
        fs::read(&temp_path).map_err(|err| format!("Failed to read thumbnail output: {}", err));
    let _ = fs::remove_file(&temp_path);
    let bytes = bytes_result?;
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("Thumbnail output was not a valid PNG file.".to_string());
    }

    let response_mode = args
        .get("response_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("url");
    let as_data_url =
        |bytes: &[u8]| format!("data:image/png;base64,{}", BASE64_STANDARD.encode(bytes));

    match (response_mode, thumbnail_cache) {
        ("data_url", _) | (_, None) => Ok(as_data_url(&bytes)),
        (_, Some(cache)) => {
            let source = args
                .get("uuid")
                .or_else(|| args.get("path"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let (_cache_key, url) = cache.save_thumbnail(source, &bytes)?;
            Ok(url)
        }
    }
}

fn temp_thumbnail_path() -> Result<PathBuf, String> {
    let mut path = env::temp_dir();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("Failed to read system time: {}", err))?
        .as_millis();
    let pid = std::process::id();
    path.push(format!("pcli2-thumbnail-{}-{}.png", pid, timestamp));
    Ok(path)
}

// ---------------------------------------------------------------------------
// Convenience wrappers used by integration tests
// ---------------------------------------------------------------------------

pub async fn run_pcli2_tenant_list(args: Value) -> Result<String, String> {
    let spec = find_tool("pcli2_tenant_list").expect("tenant list tool exists");
    run_spec(spec, &args).await
}

pub async fn run_pcli2_version() -> Result<String, String> {
    let spec = find_tool("pcli2_version").expect("version tool exists");
    run_spec(spec, &Value::Null).await
}

// ---------------------------------------------------------------------------
// Process execution
// ---------------------------------------------------------------------------

pub fn shell_escape_arg(arg: &str) -> String {
    let safe = arg
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | ':' | '='));
    if safe && !arg.is_empty() {
        return arg.to_string();
    }

    if arg.is_empty() {
        return "''".to_string();
    }

    let mut escaped = String::from("'");
    for ch in arg.chars() {
        if ch == '\'' {
            escaped.push_str("'\"'\"'");
        } else {
            escaped.push(ch);
        }
    }
    escaped.push('\'');
    escaped
}

pub async fn read_limited<R: AsyncRead + Unpin>(
    mut reader: R,
    limit: usize,
    label: &str,
) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut chunk)
            .await
            .map_err(|err| format!("Failed to read pcli2 {}: {}", label, err))?;
        if read == 0 {
            break;
        }
        if buf.len() + read > limit {
            return Err(format!(
                "pcli2 {} exceeded maximum output size of {} bytes",
                label, limit
            ));
        }
        buf.extend_from_slice(&chunk[..read]);
    }
    Ok(buf)
}

/// Renders pcli2's stderr for humans. With `PCLI2_ERROR_FORMAT=json` each
/// line is a JSON object; anything that is not is passed through unchanged.
pub fn format_stderr(stderr: &str) -> String {
    stderr
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            if let Ok(Value::Object(obj)) = serde_json::from_str::<Value>(line.trim())
                && let Some(message) = obj.get("message").and_then(|m| m.as_str())
            {
                let level = obj.get("level").and_then(|l| l.as_str()).unwrap_or("ERROR");
                return match obj.get("kind").and_then(|k| k.as_str()) {
                    Some(kind) => format!("{} ({}): {}", level, kind, message),
                    None => format!("{}: {}", level, message),
                };
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub async fn run_pcli2_command(cmd_args: Vec<String>, label: &str) -> Result<String, String> {
    let rendered = cmd_args
        .iter()
        .map(|arg| shell_escape_arg(arg))
        .collect::<Vec<_>>()
        .join(" ");
    info!("▶ pcli2 {}", rendered);
    let mut command = tokio::process::Command::new(pcli2_executable());
    command
        .args(&cmd_args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in PCLI2_CHILD_ENV {
        command.env(key, value);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("Failed to execute pcli2: {}", e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture pcli2 stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture pcli2 stderr".to_string())?;

    let stdout_task = tokio::spawn(read_limited(stdout, MAX_PCLI2_OUTPUT_BYTES, "stdout"));
    let stderr_task = tokio::spawn(read_limited(stderr, MAX_PCLI2_OUTPUT_BYTES, "stderr"));

    let output = tokio::time::timeout(PCLI2_TIMEOUT, async {
        let status = child
            .wait()
            .await
            .map_err(|err| format!("Failed waiting for pcli2: {}", err))?;
        let stdout = stdout_task
            .await
            .map_err(|err| format!("Failed to read pcli2 stdout: {}", err))??;
        let stderr = stderr_task
            .await
            .map_err(|err| format!("Failed to read pcli2 stderr: {}", err))??;
        Ok((status, stdout, stderr))
    })
    .await;

    let (status, stdout, stderr) = match output {
        Ok(Ok(output)) => output,
        Ok(Err(message)) => {
            let _ = child.kill().await;
            return Err(message);
        }
        Err(_) => {
            let _ = child.kill().await;
            return Err(format!(
                "{} failed: timed out after {:?}",
                label, PCLI2_TIMEOUT
            ));
        }
    };

    let stdout = String::from_utf8_lossy(&stdout);
    let stderr = String::from_utf8_lossy(&stderr);

    if status.success() {
        Ok(stdout.trim_end().to_string())
    } else {
        let code = status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "signal".to_string());
        let mut message = format!("{} failed (exit {})", label, code);
        let details = format_stderr(&stderr);
        if !details.is_empty() {
            message.push_str(":\n");
            message.push_str(&details);
        }
        let stdout = stdout.trim_end();
        if !stdout.is_empty() {
            message.push_str("\n--- stdout ---\n");
            message.push_str(stdout);
        }
        Err(message)
    }
}

pub fn pcli2_executable() -> String {
    env::var(PCLI2_BIN_ENV).unwrap_or_else(|_| "pcli2".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_escape_arg() {
        assert_eq!(shell_escape_arg("simple"), "simple");
        assert_eq!(shell_escape_arg(""), "''");
        assert_eq!(shell_escape_arg("with space"), "'with space'");
        assert_eq!(shell_escape_arg("with'quote"), "'with'\"'\"'quote'");
    }

    #[test]
    fn test_format_stderr_json_lines() {
        let stderr = r#"{"code":67,"kind":"not_found","level":"ERROR","message":"API error: Folder not found: /x"}
plain text line
{"level":"WARN","message":"something"}"#;
        let formatted = format_stderr(stderr);
        assert_eq!(
            formatted,
            "ERROR (not_found): API error: Folder not found: /x\nplain text line\nWARN: something"
        );
    }

    #[test]
    fn test_format_stderr_plain() {
        assert_eq!(format_stderr("❌ Error: boom\n"), "❌ Error: boom");
        assert_eq!(format_stderr(""), "");
    }

    #[test]
    fn test_normalize_tenant_use_alias() {
        let spec = find_tool("pcli2_tenant_use").unwrap();
        let args = normalize_args(spec, json!({"tenant_name": "demo"}));
        assert_eq!(args["name"], "demo");
        let args = normalize_args(spec, json!({"name": "a", "tenant_name": "b"}));
        assert_eq!(args["name"], "a");
    }

    #[test]
    fn test_tool_list_contains_every_spec_once() {
        let tools = tool_list();
        assert_eq!(tools.len(), TOOLS.len());
        let mut names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), TOOLS.len(), "duplicate tool names");
    }

    #[test]
    fn test_special_tools_have_specs() {
        for name in [
            TOOL_LEGACY_LIST,
            TOOL_ASSET_THUMBNAIL,
            TOOL_THUMBNAIL_CACHE_CLEANUP,
        ] {
            assert!(find_tool(name).is_some(), "{} missing from TOOLS", name);
        }
    }

    #[tokio::test]
    async fn test_call_tool_unknown() {
        let err = call_tool(json!({"name": "nope"}), None).await.unwrap_err();
        assert!(err.contains("Unknown tool"));
    }

    #[tokio::test]
    async fn test_call_tool_missing_name() {
        let err = call_tool(json!({}), None).await.unwrap_err();
        assert!(err.contains("Missing tool name"));
    }

    #[tokio::test]
    async fn test_call_tool_argument_error_does_not_spawn() {
        // Missing required argument is rejected before pcli2 would run.
        let err = call_tool(json!({"name": "pcli2_asset_get", "arguments": {}}), None)
            .await
            .unwrap_err();
        assert!(err.contains("provide one of 'uuid' or 'path'"), "{}", err);
    }

    #[tokio::test]
    async fn test_legacy_list_invalid_resource() {
        let err = call_tool(
            json!({"name": "pcli2", "arguments": {"resource": "tenant"}}),
            None,
        )
        .await
        .unwrap_err();
        assert!(err.contains("resource"));
    }

    #[tokio::test]
    async fn test_thumbnail_cache_cleanup_without_cache() {
        let result = call_tool(json!({"name": TOOL_THUMBNAIL_CACHE_CLEANUP}), None)
            .await
            .unwrap();
        assert_eq!(
            result["content"][0]["text"],
            "Thumbnail cache is not available"
        );
    }
}
