use super::protocol::*;
use super::tools::{execute_tool, get_available_tools};
use crate::instance_manager::list_instances;
use crate::load_order::mod_info::scan_all_installed_mods;
use crate::patch_generator::get_master_patch_status;
use crate::sandbox::{find_running_pz_pid, translate_log_error, TranslatedErrorPayload};
use crate::vfs::scan_conflicts;
use serde_json::{json, Value};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

pub fn get_available_resources() -> Vec<McpResource> {
    vec![
        McpResource {
            uri: "pz://monitor/console-log".to_string(),
            name: "Project Zomboid Console Log (console.txt)".to_string(),
            description: Some("Current console.txt log from the user Zomboid directory".to_string()),
            mime_type: Some("text/plain".to_string()),
        },
        McpResource {
            uri: "pz://mods/installed-summary".to_string(),
            name: "Installed Mods Summary".to_string(),
            description: Some("Summary of all detected mods and active load order".to_string()),
            mime_type: Some("application/json".to_string()),
        },
        McpResource {
            uri: "pz://paths/config".to_string(),
            name: "Project Zomboid Detected Paths".to_string(),
            description: Some("Auto-detected game installation, workshop, and user directories".to_string()),
            mime_type: Some("application/json".to_string()),
        },
        McpResource {
            uri: "pz://patches/status".to_string(),
            name: "Master Patch Package Status".to_string(),
            description: Some("Status of the active Z_PZModStudio Master Patch and draft resolutions".to_string()),
            mime_type: Some("application/json".to_string()),
        },
        McpResource {
            uri: "pz://game/status".to_string(),
            name: "Game Process & Bridge Status".to_string(),
            description: Some("Live execution state, Process ID (PID), and companion bridge mod status".to_string()),
            mime_type: Some("application/json".to_string()),
        },
        McpResource {
            uri: "pz://profiles/list".to_string(),
            name: "Mod Profiles Snapshot".to_string(),
            description: Some("List of all saved mod profiles/instances, custom load orders, and active profile".to_string()),
            mime_type: Some("application/json".to_string()),
        },
        McpResource {
            uri: "pz://conflicts/active".to_string(),
            name: "Active VFS Conflicts".to_string(),
            description: Some("Virtual File System file overlap and collision snapshot between active mods".to_string()),
            mime_type: Some("application/json".to_string()),
        },
        McpResource {
            uri: "pz://diagnostics/latest-crash".to_string(),
            name: "Latest Crash Diagnostics".to_string(),
            description: Some("Real-time parsed error cards, exceptions, and suggested solutions from console.txt".to_string()),
            mime_type: Some("application/json".to_string()),
        },
    ]
}

pub fn read_resource_by_uri(uri: &str) -> Result<ResourceReadResult, String> {
    let paths = crate::vfs::studio_paths();

    match uri {
        "pz://monitor/console-log" => {
            let console_path = Path::new(&paths.user_zomboid_dir).join("console.txt");
            if !console_path.exists() {
                return Err(format!("console.txt not found at: {}", console_path.display()));
            }
            // The last 2000 lines, read from the end of the file, to keep the payload small.
            let truncated = crate::sandbox::tail_lines(&console_path, 2000)
                .map_err(|e| format!("Failed to read console.txt: {}", e))?
                .join("\n");

            Ok(ResourceReadResult {
                contents: vec![ResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("text/plain".to_string()),
                    text: Some(truncated),
                }],
            })
        }
        "pz://mods/installed-summary" => {
            let manifests = scan_all_installed_mods(&paths);
            let json_str = serde_json::to_string_pretty(&json!({
                "total_installed": manifests.iter().filter(|m| !m.not_installed).count(),
                "mods": manifests
            })).map_err(|e| e.to_string())?;

            Ok(ResourceReadResult {
                contents: vec![ResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("application/json".to_string()),
                    text: Some(json_str),
                }],
            })
        }
        "pz://paths/config" => {
            let json_str = serde_json::to_string_pretty(&paths).map_err(|e| e.to_string())?;
            Ok(ResourceReadResult {
                contents: vec![ResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("application/json".to_string()),
                    text: Some(json_str),
                }],
            })
        }
        "pz://patches/status" => {
            let status = get_master_patch_status(&paths.user_zomboid_dir, &paths.mod_list_ini_path, None);
            let json_str = serde_json::to_string_pretty(&status).map_err(|e| e.to_string())?;
            Ok(ResourceReadResult {
                contents: vec![ResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("application/json".to_string()),
                    text: Some(json_str),
                }],
            })
        }
        "pz://game/status" => {
            let pid = find_running_pz_pid();
            let bridge_installed = Path::new(&paths.user_zomboid_dir).join("mods").join("Z_PZModStudio_Bridge").exists();
            let json_str = serde_json::to_string_pretty(&json!({
                "is_running": pid.is_some(),
                "pid": pid,
                "bridge_mod_installed": bridge_installed
            })).map_err(|e| e.to_string())?;

            Ok(ResourceReadResult {
                contents: vec![ResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("application/json".to_string()),
                    text: Some(json_str),
                }],
            })
        }
        "pz://profiles/list" => {
            let profiles = list_instances(paths.user_zomboid_dir.clone()).unwrap_or_default();
            let json_str = serde_json::to_string_pretty(&json!({
                "total_profiles": profiles.len(),
                "profiles": profiles
            })).map_err(|e| e.to_string())?;

            Ok(ResourceReadResult {
                contents: vec![ResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("application/json".to_string()),
                    text: Some(json_str),
                }],
            })
        }
        "pz://conflicts/active" => {
            let conflicts = scan_conflicts(&paths);
            let json_str = serde_json::to_string_pretty(&json!({
                "total_conflicts": conflicts.len(),
                "conflicts": conflicts
            })).map_err(|e| e.to_string())?;

            Ok(ResourceReadResult {
                contents: vec![ResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("application/json".to_string()),
                    text: Some(json_str),
                }],
            })
        }
        "pz://diagnostics/latest-crash" => {
            let console_path = Path::new(&paths.user_zomboid_dir).join("console.txt");
            let mut error_cards: Vec<TranslatedErrorPayload> = Vec::new();
            if console_path.exists() {
                if let Ok(file) = File::open(&console_path) {
                    let reader = BufReader::new(file);
                    let mut counter = 1;
                    for line_res in reader.lines() {
                        if let Ok(line) = line_res {
                            if let Some(card) = translate_log_error(&line, counter) {
                                error_cards.push(card);
                                counter += 1;
                            }
                        }
                    }
                }
            }
            let pid = find_running_pz_pid();
            let json_str = serde_json::to_string_pretty(&json!({
                "game_running": pid.is_some(),
                "pid": pid,
                "diagnosed_error_cards_count": error_cards.len(),
                "diagnosed_error_cards": error_cards
            })).map_err(|e| e.to_string())?;

            Ok(ResourceReadResult {
                contents: vec![ResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("application/json".to_string()),
                    text: Some(json_str),
                }],
            })
        }
        _ => Err(format!("Unknown resource URI: '{}'", uri)),
    }
}

/// Runs the standard MCP JSON-RPC stdio server loop.
pub fn run_stdio_server() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = stdin.lock();

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => break,
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match parse_request(trimmed) {
            Ok(req) => req,
            Err(err_res) => {
                let _ = writeln!(stdout, "{}", serde_json::to_string(&err_res).unwrap());
                let _ = stdout.flush();
                continue;
            }
        };

        // Handle notifications (no id)
        if request.id.is_none() {
            // Notifications like "notifications/initialized" require no response
            continue;
        }

        let response = handle_request(request);
        if let Ok(resp_str) = serde_json::to_string(&response) {
            let _ = writeln!(stdout, "{}", resp_str);
            let _ = stdout.flush();
        }
    }
}

/// One line from the client as a request, or the error to answer it with: -32700 when it is not
/// JSON, -32600 when it is JSON but not a request. The id is null unless it could be read.
fn parse_request(line: &str) -> Result<JsonRpcRequest, JsonRpcResponse> {
    let fail = |id: Option<Value>, code: i64, message: String| JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: Some(id.unwrap_or(Value::Null)),
        result: None,
        error: Some(JsonRpcError { code, message, data: None }),
    };
    let value: Value = serde_json::from_str(line).map_err(|e| fail(None, -32700, format!("Parse error: {}", e)))?;
    let id = value.get("id").cloned();
    serde_json::from_value(value).map_err(|e| fail(id, -32600, format!("Invalid request: {}", e)))
}

fn handle_request(req: JsonRpcRequest) -> JsonRpcResponse {
    let req_id = req.id.clone();

    match req.method.as_str() {
        "initialize" => {
            let result = InitializeResult {
                protocol_version: "2024-11-05".to_string(),
                capabilities: ServerCapabilities {
                    tools: Some(json!({})),
                    resources: Some(json!({})),
                    // No prompts are served, so none are offered.
                    prompts: None,
                },
                server_info: ServerInfo {
                    name: "pz-mod-studio-mcp".to_string(),
                    version: "0.1.0".to_string(),
                },
            };
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req_id,
                result: Some(serde_json::to_value(result).unwrap_or(Value::Null)),
                error: None,
            }
        }
        "ping" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req_id,
            result: Some(json!({})),
            error: None,
        },
        "tools/list" => {
            let tools = get_available_tools();
            let result = ToolsListResult { tools };
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req_id,
                result: Some(serde_json::to_value(result).unwrap_or(Value::Null)),
                error: None,
            }
        }
        "tools/call" => {
            let params = req.params.unwrap_or(json!({}));
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let tool_args = params.get("arguments").cloned().unwrap_or(json!({}));

            let tool_result = execute_tool(tool_name, tool_args);
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req_id,
                result: Some(serde_json::to_value(tool_result).unwrap_or(Value::Null)),
                error: None,
            }
        }
        "resources/list" => {
            let resources = get_available_resources();
            let result = ResourcesListResult { resources };
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req_id,
                result: Some(serde_json::to_value(result).unwrap_or(Value::Null)),
                error: None,
            }
        }
        "resources/read" => {
            let params = req.params.unwrap_or(json!({}));
            let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");

            match read_resource_by_uri(uri) {
                Ok(res) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req_id,
                    result: Some(serde_json::to_value(res).unwrap_or(Value::Null)),
                    error: None,
                },
                Err(err_msg) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req_id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: err_msg,
                        data: None,
                    }),
                },
            }
        }
        _ => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req_id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Method not found: '{}'", req.method),
                data: None,
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreadable_lines_are_answered_with_a_null_id() {
        let not_json = serde_json::to_value(parse_request("{nope").unwrap_err()).unwrap();
        assert_eq!(not_json["id"], Value::Null);
        assert_eq!(not_json["error"]["code"], -32700);
        assert!(not_json.as_object().unwrap().contains_key("id"));

        let not_a_request = serde_json::to_value(parse_request(r#"{"jsonrpc":"2.0","id":7}"#).unwrap_err()).unwrap();
        assert_eq!(not_a_request["id"], 7);
        assert_eq!(not_a_request["error"]["code"], -32600);

        assert!(parse_request(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#).is_ok());
    }

    #[test]
    fn only_what_is_served_is_offered() {
        let init = handle_request(parse_request(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#).unwrap());
        let caps = &init.result.unwrap()["capabilities"];
        assert!(caps.get("tools").is_some() && caps.get("resources").is_some());
        assert!(caps.get("prompts").is_none());
    }
}
