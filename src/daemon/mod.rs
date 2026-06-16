use crate::app::build_session;
use crate::core::{DecisionRecord, ExternalDecisionInput, LogEntry, PendingDecision, WorldState};
use crate::tui::UiEvent;
use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use crate::core::RuntimeSession;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionSnapshot {
    pub run_id: String,
    pub scenario_id: String,
    pub scenario_version: String,
    pub config_hash: String,
    pub active_locale: String,
    pub tick: u64,
    pub day: u32,
    pub paused: bool,
    pub presentation_frame: u64,
    pub outcome: Option<String>,
    pub pending_decision: Option<PendingDecision>,
    pub state: WorldState,
    pub logs: Vec<LogEntry>,
    pub decisions: Vec<DecisionRecord>,
    pub ui_events: Vec<UiEvent>,
    pub counts: SessionCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionCounts {
    pub domain_events: usize,
    pub decisions: usize,
    pub logs: usize,
    pub ui_events: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonRequest {
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonResponse {
    pub id: u64,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartRunArgs {
    pub scenario: Option<String>,
    pub locale: Option<String>,
    pub seed: Option<u64>,
    pub db_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StepArgs {
    pub ticks: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetStateArgs {
    pub advance_frame: Option<bool>,
}

#[derive(Default)]
pub struct SessionHost {
    session: Option<RuntimeSession>,
}

impl SessionHost {
    pub fn with_session(session: RuntimeSession) -> Self {
        Self {
            session: Some(session),
        }
    }

    pub fn handle(&mut self, method: &str, params: Value) -> Result<Value> {
        match method {
            "start_run" => self.start_run(params),
            "get_state" => {
                let args: GetStateArgs = from_value_or_empty(params)?;
                let session = self.session_mut()?;
                if args.advance_frame.unwrap_or(false) {
                    session.advance_presentation_frame();
                }
                Ok(serde_json::to_value(session_snapshot(session))?)
            }
            "step" => {
                let args: StepArgs = from_value_or_empty(params)?;
                let ticks = args.ticks.unwrap_or(1).max(1);
                self.session_mut()?.run_ticks(ticks)?;
                Ok(serde_json::to_value(session_snapshot(self.session()?))?)
            }
            "get_pending_decision" => {
                let session = self.session()?;
                Ok(json!({
                    "pending_decision": session.pending_decision(),
                    "state": session_snapshot(session)
                }))
            }
            "submit_decision" => {
                let input: ExternalDecisionInput = from_value_or_empty(params)?;
                let report = self.session_mut()?.submit_external_decision(input)?;
                Ok(json!({
                    "state": session_snapshot(self.session()?),
                    "report": {
                        "events": report.events,
                        "decisions": report.decisions,
                        "logs": report.logs,
                        "ui_events": report.ui_events,
                        "snapshot_saved": report.snapshot_saved
                    }
                }))
            }
            "pause" => {
                let session = self.session_mut()?;
                if !session.paused {
                    session.toggle_pause()?;
                }
                Ok(serde_json::to_value(session_snapshot(self.session()?))?)
            }
            "resume" => {
                let session = self.session_mut()?;
                if session.paused {
                    session.toggle_pause()?;
                }
                Ok(serde_json::to_value(session_snapshot(self.session()?))?)
            }
            _ => Err(anyhow!("unknown daemon method {method}")),
        }
    }

    fn start_run(&mut self, params: Value) -> Result<Value> {
        let args: StartRunArgs = from_value_or_empty(params)?;
        let db_path = args.db_path.map(PathBuf::from);
        let session = build_session(
            args.scenario.as_deref().unwrap_or("sea_survival"),
            args.locale.as_deref().unwrap_or("zh-CN"),
            args.seed.unwrap_or(42),
            db_path,
        )?;
        let snapshot = session_snapshot(&session);
        self.session = Some(session);
        Ok(serde_json::to_value(snapshot)?)
    }

    fn session(&self) -> Result<&RuntimeSession> {
        self.session
            .as_ref()
            .ok_or_else(|| anyhow!("no active Waves session; call waves_start_run first"))
    }

    fn session_mut(&mut self) -> Result<&mut RuntimeSession> {
        self.session
            .as_mut()
            .ok_or_else(|| anyhow!("no active Waves session; call waves_start_run first"))
    }
}

pub fn session_snapshot(session: &RuntimeSession) -> SessionSnapshot {
    SessionSnapshot {
        run_id: session.run_id.clone(),
        scenario_id: session.config.manifest.id.clone(),
        scenario_version: session.config.manifest.version.clone(),
        config_hash: session.config.config_hash.clone(),
        active_locale: session.catalog.active_locale().to_string(),
        tick: session.state.tick,
        day: session.state.environment.day,
        paused: session.paused,
        presentation_frame: session.presentation_frame,
        outcome: session.state.outcome.clone(),
        pending_decision: session.pending_decision().cloned(),
        state: session.state.clone(),
        logs: session.logs.clone(),
        decisions: session.decisions.clone(),
        ui_events: session.ui_events.clone(),
        counts: SessionCounts {
            domain_events: session.domain_events.len(),
            decisions: session.decisions.len(),
            logs: session.logs.len(),
            ui_events: session.ui_events.len(),
        },
    }
}

pub fn run_server(
    scenario: &str,
    locale: &str,
    seed: u64,
    db_path: Option<PathBuf>,
    socket_path: PathBuf,
) -> Result<()> {
    prepare_socket(&socket_path)?;
    let session = build_session(scenario, locale, seed, db_path)?;
    let mut host = SessionHost::with_session(session);
    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("binding socket {}", socket_path.display()))?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = handle_client(stream, &mut host) {
                    eprintln!("daemon client error: {error:#}");
                }
            }
            Err(error) => {
                eprintln!("daemon accept error: {error}");
            }
        }
    }
    Ok(())
}

fn prepare_socket(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating socket dir {}", parent.display()))?;
    }
    if path.exists() {
        if UnixStream::connect(path).is_ok() {
            bail!("socket {} is already serving", path.display());
        }
        fs::remove_file(path)
            .with_context(|| format!("removing stale socket {}", path.display()))?;
    }
    Ok(())
}

fn handle_client(stream: UnixStream, host: &mut SessionHost) -> Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;
    let mut line = String::new();
    reader.read_line(&mut line)?;
    if line.trim().is_empty() {
        return Ok(());
    }
    let request: DaemonRequest = serde_json::from_str(&line)
        .with_context(|| format!("parsing daemon request {}", line.trim()))?;
    let response = match host.handle(&request.method, request.params) {
        Ok(result) => DaemonResponse {
            id: request.id,
            ok: true,
            result: Some(result),
            error: None,
        },
        Err(error) => DaemonResponse {
            id: request.id,
            ok: false,
            result: None,
            error: Some(error.to_string()),
        },
    };
    writeln!(writer, "{}", serde_json::to_string(&response)?)?;
    writer.flush()?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct DaemonClient {
    socket_path: PathBuf,
}

impl DaemonClient {
    pub fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: socket_path.into(),
        }
    }

    pub fn request_value(&self, method: &str, params: Value) -> Result<Value> {
        let request = DaemonRequest {
            id: 1,
            method: method.to_string(),
            params,
        };
        let mut stream = UnixStream::connect(&self.socket_path)
            .with_context(|| format!("connecting {}", self.socket_path.display()))?;
        writeln!(stream, "{}", serde_json::to_string(&request)?)?;
        stream.flush()?;

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line.trim().is_empty() {
            bail!("daemon closed connection without response");
        }
        let response: DaemonResponse = serde_json::from_str(&line)
            .with_context(|| format!("parsing daemon response {}", line.trim()))?;
        if !response.ok {
            bail!(
                "{}",
                response
                    .error
                    .unwrap_or_else(|| "daemon request failed".to_string())
            );
        }
        response
            .result
            .ok_or_else(|| anyhow!("daemon response missing result"))
    }

    pub fn request<T: for<'de> Deserialize<'de>>(&self, method: &str, params: Value) -> Result<T> {
        serde_json::from_value(self.request_value(method, params)?).map_err(Into::into)
    }
}

pub fn from_value_or_empty<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T> {
    if value.is_null() {
        serde_json::from_value(json!({})).map_err(Into::into)
    } else {
        serde_json::from_value(value).map_err(Into::into)
    }
}
