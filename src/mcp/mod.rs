use crate::daemon::{DaemonClient, SessionHost};
use anyhow::{Result, anyhow};
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

pub fn run_stdio(connect: Option<PathBuf>) -> Result<()> {
    run_stdio_with_scenarios_dir(connect, None)
}

pub fn run_stdio_with_scenarios_dir(
    connect: Option<PathBuf>,
    scenarios_dir: Option<PathBuf>,
) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut server = McpServer::new(connect, scenarios_dir);

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let request = match serde_json::from_str::<RpcRequest>(&line) {
            Ok(request) => request,
            Err(error) => {
                write_rpc_error(
                    &mut stdout,
                    Value::Null,
                    -32700,
                    &format!("parse error: {error}"),
                )?;
                continue;
            }
        };
        let Some(id) = request.id.clone() else {
            server.handle_notification(&request.method);
            continue;
        };
        match server.handle_request(&request) {
            Ok(result) => write_rpc_result(&mut stdout, id, result)?,
            Err(error) => write_rpc_error(&mut stdout, id, -32603, &error.to_string())?,
        }
    }

    Ok(())
}

struct McpServer {
    backend: McpBackend,
}

enum McpBackend {
    Local(Box<SessionHost>),
    Remote(DaemonClient),
}

impl McpServer {
    fn new(connect: Option<PathBuf>, scenarios_dir: Option<PathBuf>) -> Self {
        let backend = match connect {
            Some(path) => McpBackend::Remote(DaemonClient::new(path)),
            None => McpBackend::Local(Box::new(SessionHost::with_scenarios_dir(scenarios_dir))),
        };
        Self { backend }
    }

    fn handle_notification(&mut self, _method: &str) {}

    fn handle_request(&mut self, request: &RpcRequest) -> Result<Value> {
        match request.method.as_str() {
            "initialize" => Ok(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "waves",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
            "tools/list" => Ok(json!({ "tools": tools() })),
            "tools/call" => self.handle_tool_call(request.params.clone().unwrap_or(Value::Null)),
            method => Err(anyhow!("unknown method {method}")),
        }
    }

    fn handle_tool_call(&mut self, params: Value) -> Result<Value> {
        let call: ToolCall = serde_json::from_value(params)
            .map_err(|error| anyhow!("invalid tools/call params: {error}"))?;
        match self.execute_tool(&call.name, call.arguments.unwrap_or(Value::Null)) {
            Ok(value) => Ok(tool_result(value, false)),
            Err(error) => Ok(tool_result(json!({ "error": error.to_string() }), true)),
        }
    }

    fn execute_tool(&mut self, name: &str, arguments: Value) -> Result<Value> {
        let method = match name {
            "waves_start_run" => "start_run",
            "waves_get_state" => "get_state",
            "waves_step" => "step",
            "waves_get_pending_decision" => "get_pending_decision",
            "waves_submit_decision" => "submit_decision",
            "waves_pause" => "pause",
            "waves_resume" => "resume",
            _ => return Err(anyhow!("unknown tool {name}")),
        };
        let raw = match &mut self.backend {
            McpBackend::Local(host) => host.handle(method, arguments),
            McpBackend::Remote(client) => {
                let result = client.request_value(method, arguments);
                let _ = client.request_value("record_agent_activity", json!({ "tool": name }));
                result
            }
        }?;
        Ok(compact_tool_output(name, raw))
    }
}

fn compact_tool_output(tool_name: &str, value: Value) -> Value {
    match tool_name {
        "waves_start_run" | "waves_get_state" | "waves_step" | "waves_pause" | "waves_resume" => {
            compact_snapshot(&value)
        }
        "waves_get_pending_decision" => json!({
            "pending_decision": compact_pending_decision(&value["pending_decision"]),
            "state": compact_snapshot(&value["state"]),
        }),
        "waves_submit_decision" => json!({
            "report": compact_report(&value["report"]),
            "state": compact_snapshot(&value["state"]),
        }),
        _ => value,
    }
}

fn compact_snapshot(snapshot: &Value) -> Value {
    json!({
        "run_id": snapshot["run_id"],
        "scenario_id": snapshot["scenario_id"],
        "scenario_version": snapshot["scenario_version"],
        "config_hash": snapshot["config_hash"],
        "active_locale": snapshot["active_locale"],
        "tick": snapshot["tick"],
        "day": snapshot["day"],
        "paused": snapshot["paused"],
        "outcome": snapshot["outcome"],
        "agent_connection": snapshot["agent_connection"],
        "state": compact_world_state(&snapshot["state"]),
        "pending_decision": compact_pending_decision(&snapshot["pending_decision"]),
        "recent_logs": recent_items(&snapshot["logs"], 5),
        "recent_decisions": recent_items(&snapshot["decisions"], 5),
        "counts": snapshot["counts"],
    })
}

fn compact_world_state(state: &Value) -> Value {
    json!({
        "tick": state["tick"],
        "alive": state["alive"],
        "outcome": state["outcome"],
        "stats": state["stats"],
        "resources": state["resources"],
        "environment": state["environment"],
        "memory": state["memory"],
        "personality": state["personality"],
    })
}

fn compact_pending_decision(pending: &Value) -> Value {
    if pending.is_null() {
        Value::Null
    } else {
        json!({
            "id": pending["id"],
            "tick": pending["tick"],
            "scenario_id": pending["scenario_id"],
            "event": pending["event"],
            "actions": pending["actions"],
            "state": compact_world_state(&pending["state"]),
        })
    }
}

fn compact_report(report: &Value) -> Value {
    let resolutions = report["events"]
        .as_array()
        .map(|events| {
            events
                .iter()
                .filter_map(|event| {
                    let resolution = &event["payload"]["resolution"];
                    if resolution.is_null() {
                        None
                    } else {
                        Some(json!({
                            "tick": event["tick"],
                            "event": event["payload"]["event"],
                            "decision": event["payload"]["decision"],
                            "success": resolution["success"],
                            "summary": resolution["summary"],
                            "changes": resolution["changes"],
                        }))
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    json!({
        "decisions": report["decisions"],
        "resolutions": resolutions,
        "recent_logs": recent_items(&report["logs"], 5),
        "snapshot_saved": report["snapshot_saved"],
        "ui_event_count": report["ui_events"].as_array().map_or(0, Vec::len),
    })
}

fn recent_items(value: &Value, limit: usize) -> Value {
    value.as_array().map_or_else(
        || Value::Array(Vec::new()),
        |items| {
            let start = items.len().saturating_sub(limit);
            Value::Array(items[start..].to_vec())
        },
    )
}

#[derive(Debug, Deserialize)]
struct RpcRequest {
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ToolCall {
    name: String,
    arguments: Option<Value>,
}

fn tool_result(value: Value, is_error: bool) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
            }
        ],
        "isError": is_error
    })
}

fn write_rpc_result(stdout: &mut io::Stdout, id: Value, result: Value) -> Result<()> {
    writeln!(
        stdout,
        "{}",
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        })
    )?;
    stdout.flush()?;
    Ok(())
}

fn write_rpc_error(stdout: &mut io::Stdout, id: Value, code: i64, message: &str) -> Result<()> {
    writeln!(
        stdout,
        "{}",
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message
            }
        })
    )?;
    stdout.flush()?;
    Ok(())
}

fn tools() -> Vec<Value> {
    vec![
        json!({
            "name": "waves_start_run",
            "description": "Start a new Waves run controlled by the external agent.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "scenario": { "type": "string", "default": "sea_survival" },
                    "locale": { "type": "string", "default": "zh-CN" },
                    "seed": { "type": "integer", "default": 42 },
                    "db_path": { "type": "string" },
                    "scenarios_dir": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "waves_get_state",
            "description": "Get the current run state, recent logs, recent decisions, and pending decision if any.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "waves_step",
            "description": "Advance the run by ticks. Stops early if a pending decision appears.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "ticks": { "type": "integer", "minimum": 1, "default": 1 }
                }
            }
        }),
        json!({
            "name": "waves_get_pending_decision",
            "description": "Get the current pending decision with event, state snapshot, and available actions.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "waves_submit_decision",
            "description": "Submit the external agent's action for the current pending decision.",
            "inputSchema": {
                "type": "object",
                "required": ["decision_id", "action_id", "reason"],
                "properties": {
                    "decision_id": { "type": "string" },
                    "action_id": { "type": "string" },
                    "reason": {
                        "type": "string",
                        "description": "Use the current run's active_locale language when possible."
                    },
                    "risk_attitude": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "waves_pause",
            "description": "Pause the active run.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "waves_resume",
            "description": "Resume the active run.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_state_omits_full_history_but_keeps_recent_context() {
        let snapshot = json!({
            "run_id": "run-1",
            "scenario_id": "sea_survival",
            "scenario_version": "0.1.0",
            "config_hash": "hash",
            "active_locale": "zh-CN",
            "tick": 12,
            "day": 1,
            "paused": false,
            "outcome": null,
            "state": {
                "tick": 12,
                "alive": true,
                "outcome": null,
                "stats": { "hunger": 50.0 },
                "resources": { "food": 1.0 },
                "environment": { "risk": "Low" },
                "memory": { "recent": [] },
                "personality": { "risk_bias": 0.0 }
            },
            "pending_decision": {
                "id": "tick-12-rain",
                "tick": 12,
                "scenario_id": "sea_survival",
                "event": { "id": "rain" },
                "actions": [{ "id": "collect_rain" }],
                "state": {
                    "tick": 12,
                    "alive": true,
                    "outcome": null,
                    "stats": { "thirst": 70.0 },
                    "resources": { "water": 1.0 },
                    "environment": { "risk": "Medium" },
                    "memory": { "recent": [] },
                    "personality": { "risk_bias": 0.0 }
                }
            },
            "logs": [1, 2, 3, 4, 5, 6],
            "decisions": [1, 2, 3, 4, 5, 6],
            "ui_events": [1, 2, 3],
            "counts": { "ui_events": 3 }
        });

        let compact = compact_tool_output("waves_get_state", snapshot);

        assert!(compact.get("ui_events").is_none());
        assert_eq!(compact["recent_logs"], json!([2, 3, 4, 5, 6]));
        assert_eq!(compact["recent_decisions"], json!([2, 3, 4, 5, 6]));
        assert_eq!(
            compact["pending_decision"]["state"]["stats"]["thirst"],
            70.0
        );
        assert!(
            compact["pending_decision"]["state"]
                .get("ui_events")
                .is_none()
        );
    }

    #[test]
    fn compact_submit_response_keeps_resolution_summary() {
        let submit = json!({
            "report": {
                "decisions": [{ "action_id": "eat_food" }],
                "events": [{
                    "tick": 4,
                    "payload": {
                        "event": { "id": "heat" },
                        "decision": { "choice": "eat_food" },
                        "resolution": {
                            "success": true,
                            "summary": "饥饿 -16",
                            "changes": [{ "target": "hunger", "delta": -16.0 }]
                        }
                    }
                }],
                "logs": [1, 2, 3, 4, 5, 6],
                "snapshot_saved": false,
                "ui_events": [1, 2, 3, 4]
            },
            "state": {
                "run_id": "run-1",
                "scenario_id": "sea_survival",
                "scenario_version": "0.1.0",
                "config_hash": "hash",
                "active_locale": "zh-CN",
                "tick": 4,
                "day": 1,
                "paused": false,
                "outcome": null,
                "state": {
                    "tick": 4,
                    "alive": true,
                    "outcome": null,
                    "stats": { "hunger": 30.0 },
                    "resources": { "food": 1.0 },
                    "environment": { "risk": "Low" },
                    "memory": { "recent": [] },
                    "personality": { "risk_bias": 0.0 }
                },
                "pending_decision": null,
                "logs": [],
                "decisions": [],
                "counts": {}
            }
        });

        let compact = compact_tool_output("waves_submit_decision", submit);

        assert_eq!(compact["report"]["recent_logs"], json!([2, 3, 4, 5, 6]));
        assert_eq!(compact["report"]["ui_event_count"], 4);
        assert_eq!(compact["report"]["resolutions"][0]["summary"], "饥饿 -16");
        assert!(compact["state"].get("ui_events").is_none());
    }
}
