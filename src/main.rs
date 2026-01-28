use anyhow::Result;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::io::{stdin, stdout};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use async_trait::async_trait;

#[derive(Parser)]
#[command(name = "pcli2-mcp")]
#[command(about = "MCP server for PCLI2 integration with Ollama")]
struct Args {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

// Define the basic structures for our MCP server
#[derive(Debug, Clone)]
pub struct ServerCapabilities {
    pub tools: bool,
}

impl ServerCapabilities {
    pub fn builder() -> ServerCapabilitiesBuilder {
        ServerCapabilitiesBuilder::default()
    }
}

#[derive(Default)]
pub struct ServerCapabilitiesBuilder {
    tools: bool,
}

impl ServerCapabilitiesBuilder {
    pub fn enable_tools(mut self) -> Self {
        self.tools = true;
        self
    }

    pub fn build(self) -> ServerCapabilities {
        ServerCapabilities {
            tools: self.tools,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    pub instructions: Option<String>,
    pub capabilities: ServerCapabilities,
}

#[derive(Debug, Clone)]
pub struct Tool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Option<serde_json::Value>,
}

#[derive(Debug)]
pub struct ListToolsParams {}

#[derive(Debug)]
pub struct ListToolsResult {
    pub tools: Vec<Tool>,
    pub next_cursor: Option<String>,
}

#[derive(Debug)]
pub struct CallToolParams {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug)]
pub struct ContentChunk {
    pub text: String,
}

#[derive(Debug)]
pub struct CallToolResult {
    pub content: Vec<ContentChunk>,
    pub is_error: Option<bool>,
}

#[async_trait]
pub trait McpService: Send + Sync {
    async fn list_tools(&self, params: ListToolsParams) -> Result<ListToolsResult, anyhow::Error>;
    async fn call_tool(&self, params: CallToolParams) -> Result<CallToolResult, anyhow::Error>;
}

/// MCP server for PCLI2 integration
struct Pcli2McpService {
    tools: HashMap<String, Tool>,
}

#[derive(Serialize, Deserialize)]
struct Pcli2Args {
    command: String,
    subcommand: Option<String>,
    args: Vec<String>,
}

impl Pcli2McpService {
    fn new() -> Self {
        let mut tools = HashMap::new();

        // Define PCLI2 tools based on available commands
        tools.insert(
            "pcli2_tenant".to_string(),
            Tool {
                name: "pcli2_tenant".to_string(),
                description: Some("Manage tenants in PCLI2".to_string()),
                input_schema: None,
            },
        );

        tools.insert(
            "pcli2_folder".to_string(),
            Tool {
                name: "pcli2_folder".to_string(),
                description: Some("Manage folders in PCLI2".to_string()),
                input_schema: None,
            },
        );

        tools.insert(
            "pcli2_auth".to_string(),
            Tool {
                name: "pcli2_auth".to_string(),
                description: Some("Authentication operations in PCLI2".to_string()),
                input_schema: None,
            },
        );

        tools.insert(
            "pcli2_asset".to_string(),
            Tool {
                name: "pcli2_asset".to_string(),
                description: Some("Manage assets in PCLI2".to_string()),
                input_schema: None,
            },
        );

        tools.insert(
            "pcli2_config".to_string(),
            Tool {
                name: "pcli2_config".to_string(),
                description: Some("Configuration management in PCLI2".to_string()),
                input_schema: None,
            },
        );

        Self { tools }
    }
}

#[async_trait]
impl McpService for Pcli2McpService {
    async fn list_tools(&self, _params: ListToolsParams) -> Result<ListToolsResult, anyhow::Error> {
        info!("Listing available PCLI2 tools");
        let tools: Vec<Tool> = self.tools.values().cloned().collect();
        Ok(ListToolsResult {
            tools,
            next_cursor: None
        })
    }

    async fn call_tool(&self, params: CallToolParams) -> Result<CallToolResult, anyhow::Error> {
        info!("Calling tool: {}", params.name);

        // Parse the arguments
        let args: Pcli2Args = serde_json::from_value(params.arguments)
            .unwrap_or(Pcli2Args {
                command: params.name.replace("pcli2_", ""),
                subcommand: None,
                args: vec![],
            });

        // Construct the command
        let mut cmd_args = vec![args.command.clone()];
        if let Some(subcommand) = args.subcommand {
            cmd_args.push(subcommand);
        }
        cmd_args.extend(args.args);

        // Execute the PCLI2 command
        let output = tokio::process::Command::new("pcli2")
            .args(&cmd_args)
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to execute pcli2 command: {}", e))?;

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let stderr_str = String::from_utf8_lossy(&output.stderr);

        let result = if output.status.success() {
            format!("PCLI2 Command Output:\n{}", stdout_str)
        } else {
            format!(
                "PCLI2 Command Error:\n{}\nStderr:\n{}",
                stdout_str, stderr_str
            )
        };

        Ok(CallToolResult {
            content: vec![ContentChunk {
                text: result,
            }],
            is_error: Some(!output.status.success()),
        })
    }
}

// Simple MCP protocol implementation
async fn serve_stdio(service: impl McpService + 'static) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use serde_json::{json, Value};

    let stdin = stdin();
    let mut stdout = stdout();
    let reader = BufReader::new(stdin);

    let mut lines = reader.lines();

    info!("MCP server listening on stdio...");

    // Process incoming lines
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        info!("Received line: {}", line);

        // Parse the JSON-RPC message
        if let Ok(value) = serde_json::from_str::<Value>(&line) {
            if let Some(id) = value.get("id") {
                if let Some(method) = value.get("method") {
                    match method.as_str() {
                        Some("initialize") => {
                            info!("Processing initialize request");
                            // Send initialization response with proper protocol version
                            let response = json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "protocolVersion": "2025-03-26",
                                    "serverInfo": {
                                        "name": "pcli2-mcp",
                                        "version": "0.1.0"
                                    },
                                    "capabilities": {
                                        "tools": {}
                                    }
                                }
                            });

                            let response_str = serde_json::to_string(&response)?;
                            info!("Sending initialize response: {}", response_str);
                            stdout.write_all(response_str.as_bytes()).await?;
                            stdout.write_all(b"\n").await?;
                            stdout.flush().await?;
                            info!("Initialize response sent");
                        }
                        Some("initialized") => {
                            // Respond to initialized notification
                            info!("Client initialized");
                        }
                        Some("tools/list") => {
                            // Handle tools/list request
                            let tools_result = service.list_tools(ListToolsParams {}).await?;

                            let tools_json: Vec<Value> = tools_result.tools.into_iter().map(|tool| {
                                json!({
                                    "name": tool.name,
                                    "description": tool.description,
                                    "inputSchema": tool.input_schema
                                })
                            }).collect();

                            let response = json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "tools": tools_json
                                }
                            });

                            let response_str = serde_json::to_string(&response)?;
                            stdout.write_all(format!("{}\n", response_str).as_bytes()).await?;
                            stdout.flush().await?;
                        }
                        Some(method_str) if method_str.starts_with("tools/") && method_str.contains("/call") => {
                            // Extract tool name from method like "tools/{name}/call"
                            let parts: Vec<&str> = method_str.split('/').collect();
                            if parts.len() >= 3 && parts[2] == "call" {
                                let tool_name = parts[1].to_string();

                                // Get the parameters
                                let arguments = if let Some(params) = value.get("params") {
                                    if let Some(args) = params.get("arguments") {
                                        args.clone()
                                    } else {
                                        json!({})
                                    }
                                } else {
                                    json!({})
                                };

                                let call_params = CallToolParams {
                                    name: tool_name,
                                    arguments,
                                };

                                // Call the tool
                                let result = service.call_tool(call_params).await?;

                                let response = json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "content": result.content.iter().map(|chunk| {
                                            json!({
                                                "type": "text",
                                                "text": chunk.text
                                            })
                                        }).collect::<Vec<_>>()
                                    }
                                });

                                let response_str = serde_json::to_string(&response)?;
                                stdout.write_all(format!("{}\n", response_str).as_bytes()).await?;
                                stdout.flush().await?;
                            }
                        }
                        _ => {
                            // Send error response for unsupported methods
                            let response = json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "error": {
                                    "code": -32601,
                                    "message": format!("Method '{}' not found", method)
                                }
                            });

                            let response_str = serde_json::to_string(&response)?;
                            stdout.write_all(format!("{}\n", response_str).as_bytes()).await?;
                            stdout.flush().await?;
                        }
                    }
                }
            } else if value.get("method").is_some() {
                // This is a notification (no id)
                if let Some(method) = value.get("method").and_then(|m| m.as_str()) {
                    match method {
                        "shutdown" => {
                            info!("Shutdown notification received");
                            break;
                        }
                        _ => {
                            info!("Unhandled notification: {}", method);
                        }
                    }
                }
            }
        } else {
            // Invalid JSON
            tracing::warn!("Invalid JSON received: {}", line);
        }
    }

    Ok(())
}

fn print_banner() {
    use std::fmt::Write;

    // Create a colorful ASCII art banner for PCLI2-MCP with warm gradient effect
    let mut banner = String::new();

    // Define ANSI color codes for warm gradient effect (yellow to red)
    let colors = [
        "\x1b[38;5;226m", // Bright yellow
        "\x1b[38;5;220m", // Orange-yellow
        "\x1b[38;5;214m", // Orange
        "\x1b[38;5;208m", // Orange-red
        "\x1b[38;5;202m", // Red-orange
        "\x1b[38;5;196m", // Bright red
    ];

    let ascii_lines = [
        "██████╗  ██████╗██╗     ██╗██████╗     ███╗   ███╗ ██████╗██████╗ ",
        "██╔══██╗██╔════╝██║     ██║╚════██╗    ████╗ ████║██╔════╝██╔══██╗",
        "██████╔╝██║     ██║     ██║ █████╔╝    ██╔████╔██║██║     ██████╔╝",
        "██╔═══╝ ██║     ██║     ██║██╔═══╝     ██║╚██╔╝██║██║     ██╔═══╝ ",
        "██║     ╚██████╗███████╗██║███████╗    ██║ ╚═╝ ██║╚██████╗██║     ",
        "╚═╝      ╚═════╝╚══════╝╚═╝╚══════╝    ╚═╝     ╚═╝ ╚═════╝╚═╝     ",
    ];

    // Print each line with warm gradient coloring (vertical gradient by row)
    for (line_idx, line) in ascii_lines.iter().enumerate() {
        let color_idx = line_idx % colors.len();
        let color = colors[color_idx];

        for ch in line.chars() {
            write!(banner, "{}{}", color, ch).unwrap();
        }
        writeln!(banner).unwrap();
    }

    // Reset color and add the uncolored subtitle
    banner.push_str("\x1b[0m");
    banner.push_str("                                                                \n");
    banner.push_str("                   MCP Server for PCLI2 Integration             \n");

    println!("{}", banner);

    // Additional info with some color - properly aligned
    println!("\x1b[36m┌─────────────────────────────────────────────────────────────┐\x1b[0m"); // Cyan
    println!("\x1b[36m│\x1b[0m  \x1b[92mConnecting PCLI2 with Local Ollama                      \x1b[0m  \x1b[36m│\x1b[0m"); // Green text
    println!("\x1b[36m│\x1b[0m  \x1b[94mvia Model Context Protocol (MCP)                        \x1b[0m  \x1b[36m│\x1b[0m"); // Blue text
    println!("\x1b[36m│\x1b[0m  \x1b[93mConnection: stdio                                       \x1b[0m  \x1b[36m│\x1b[0m"); // Yellow text
    println!("\x1b[36m└─────────────────────────────────────────────────────────────┘\x1b[0m"); // Cyan
    println!();
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Setup logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(if args.verbose { Level::DEBUG } else { Level::INFO })
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Setting default subscriber failed");

    // Print banner
    print_banner();

    info!("Starting PCLI2 MCP Server...");

    // Create the service
    let service = Pcli2McpService::new();

    // Print service info
    info!("Available tools: {:?}", service.tools.keys());

    // Start the stdio server
    serve_stdio(service).await?;

    info!("PCLI2 MCP Server stopped.");

    Ok(())
}