use crate::app::build_session_with_scenarios_dir;
use crate::core::{DecisionRecord, ExternalDecisionInput, LogEntry, PendingDecision, WorldState};
use crate::tui::UiEvent;
use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::Instant;

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
    #[serde(default)]
    pub agent_connection: AgentConnectionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionCounts {
    pub domain_events: usize,
    pub decisions: usize,
    pub logs: usize,
    pub ui_events: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentConnectionStatus {
    pub seen: bool,
    pub last_tool: Option<String>,
    pub last_active_secs_ago: Option<u64>,
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
    pub scenarios_dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StepArgs {
    pub ticks: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetStateArgs {
    pub advance_frame: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecordAgentActivityArgs {
    pub tool: Option<String>,
}

#[derive(Default)]
pub struct SessionHost {
    session: Option<RuntimeSession>,
    scenarios_dir: Option<PathBuf>,
    agent_activity: Option<AgentActivity>,
}

struct AgentActivity {
    tool: String,
    last_active: Instant,
}

impl SessionHost {
    pub fn with_session(session: RuntimeSession) -> Self {
        Self::with_session_and_scenarios_dir(session, None)
    }

    pub fn with_session_and_scenarios_dir(
        session: RuntimeSession,
        scenarios_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            session: Some(session),
            scenarios_dir,
            agent_activity: None,
        }
    }

    pub fn with_scenarios_dir(scenarios_dir: Option<PathBuf>) -> Self {
        Self {
            session: None,
            scenarios_dir,
            agent_activity: None,
        }
    }

    pub fn handle(&mut self, method: &str, params: Value) -> Result<Value> {
        match method {
            "start_run" => self.start_run(params),
            "get_state" => {
                let args: GetStateArgs = from_value_or_empty(params)?;
                {
                    let session = self.session_mut()?;
                    if args.advance_frame.unwrap_or(false) {
                        session.advance_presentation_frame();
                    }
                }
                Ok(serde_json::to_value(self.snapshot()?)?)
            }
            "step" => {
                let args: StepArgs = from_value_or_empty(params)?;
                let ticks = args.ticks.unwrap_or(1).max(1);
                self.session_mut()?.run_ticks(ticks)?;
                Ok(serde_json::to_value(self.snapshot()?)?)
            }
            "get_pending_decision" => {
                let snapshot = self.snapshot()?;
                Ok(json!({
                    "pending_decision": snapshot.pending_decision,
                    "state": snapshot
                }))
            }
            "submit_decision" => {
                let input: ExternalDecisionInput = from_value_or_empty(params)?;
                let report = self.session_mut()?.submit_external_decision(input)?;
                Ok(json!({
                    "state": self.snapshot()?,
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
                Ok(serde_json::to_value(self.snapshot()?)?)
            }
            "resume" => {
                let session = self.session_mut()?;
                if session.paused {
                    session.toggle_pause()?;
                }
                Ok(serde_json::to_value(self.snapshot()?)?)
            }
            "record_agent_activity" => {
                let args: RecordAgentActivityArgs = from_value_or_empty(params)?;
                self.record_agent_activity(args.tool.as_deref().unwrap_or("unknown"));
                Ok(json!({
                    "agent_connection": self.agent_connection_status()
                }))
            }
            _ => Err(anyhow!("unknown daemon method {method}")),
        }
    }

    fn start_run(&mut self, params: Value) -> Result<Value> {
        let args: StartRunArgs = from_value_or_empty(params)?;
        let db_path = args.db_path.map(PathBuf::from);
        let scenarios_dir = args
            .scenarios_dir
            .map(PathBuf::from)
            .or_else(|| self.scenarios_dir.clone());
        let session = build_session_with_scenarios_dir(
            args.scenario.as_deref().unwrap_or("sea_survival"),
            args.locale.as_deref().unwrap_or("zh-CN"),
            args.seed.unwrap_or(42),
            db_path,
            scenarios_dir.as_deref(),
        )?;
        self.session = Some(session);
        Ok(serde_json::to_value(self.snapshot()?)?)
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

    fn snapshot(&self) -> Result<SessionSnapshot> {
        Ok(session_snapshot_with_agent(
            self.session()?,
            self.agent_connection_status(),
        ))
    }

    fn record_agent_activity(&mut self, tool: &str) {
        self.agent_activity = Some(AgentActivity {
            tool: tool.to_string(),
            last_active: Instant::now(),
        });
    }

    fn agent_connection_status(&self) -> AgentConnectionStatus {
        self.agent_activity
            .as_ref()
            .map(|activity| AgentConnectionStatus {
                seen: true,
                last_tool: Some(activity.tool.clone()),
                last_active_secs_ago: Some(activity.last_active.elapsed().as_secs()),
            })
            .unwrap_or_default()
    }
}

pub fn session_snapshot(session: &RuntimeSession) -> SessionSnapshot {
    session_snapshot_with_agent(session, AgentConnectionStatus::default())
}

fn session_snapshot_with_agent(
    session: &RuntimeSession,
    agent_connection: AgentConnectionStatus,
) -> SessionSnapshot {
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
        agent_connection,
    }
}

pub fn run_server(
    scenario: &str,
    locale: &str,
    seed: u64,
    db_path: Option<PathBuf>,
    socket_path: PathBuf,
) -> Result<()> {
    run_server_inner(scenario, locale, seed, db_path, socket_path, None, None)
}

pub fn run_server_with_scenarios_dir(
    scenario: &str,
    locale: &str,
    seed: u64,
    db_path: Option<PathBuf>,
    socket_path: PathBuf,
    scenarios_dir: Option<PathBuf>,
) -> Result<()> {
    run_server_inner(
        scenario,
        locale,
        seed,
        db_path,
        socket_path,
        scenarios_dir,
        None,
    )
}

pub fn run_server_with_startup_status(
    scenario: &str,
    locale: &str,
    seed: u64,
    db_path: Option<PathBuf>,
    socket_path: PathBuf,
    startup: Sender<Result<(), String>>,
) -> Result<()> {
    run_server_inner(
        scenario,
        locale,
        seed,
        db_path,
        socket_path,
        None,
        Some(startup),
    )
}

pub fn run_server_with_startup_status_and_scenarios_dir(
    scenario: &str,
    locale: &str,
    seed: u64,
    db_path: Option<PathBuf>,
    socket_path: PathBuf,
    startup: Sender<Result<(), String>>,
    scenarios_dir: Option<PathBuf>,
) -> Result<()> {
    run_server_inner(
        scenario,
        locale,
        seed,
        db_path,
        socket_path,
        scenarios_dir,
        Some(startup),
    )
}

fn run_server_inner(
    scenario: &str,
    locale: &str,
    seed: u64,
    db_path: Option<PathBuf>,
    socket_path: PathBuf,
    scenarios_dir: Option<PathBuf>,
    startup: Option<Sender<Result<(), String>>>,
) -> Result<()> {
    let startup_result =
        start_listener(scenario, locale, seed, db_path, &socket_path, scenarios_dir);
    let (mut host, listener) = match startup_result {
        Ok(server) => {
            notify_startup(&startup, Ok(()));
            server
        }
        Err(error) => {
            notify_startup(&startup, Err(format!("{error:#}")));
            return Err(error);
        }
    };

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

fn start_listener(
    scenario: &str,
    locale: &str,
    seed: u64,
    db_path: Option<PathBuf>,
    socket_path: &Path,
    scenarios_dir: Option<PathBuf>,
) -> Result<(SessionHost, UnixListener)> {
    prepare_socket(socket_path)?;
    let session = build_session_with_scenarios_dir(
        scenario,
        locale,
        seed,
        db_path,
        scenarios_dir.as_deref(),
    )?;
    let host = SessionHost::with_session_and_scenarios_dir(session, scenarios_dir);
    let listener = UnixListener::bind(socket_path)
        .with_context(|| format!("binding socket {}", socket_path.display()))?;
    Ok((host, listener))
}

fn notify_startup(startup: &Option<Sender<Result<(), String>>>, status: Result<(), String>) {
    if let Some(startup) = startup {
        let _ = startup.send(status);
    }
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
