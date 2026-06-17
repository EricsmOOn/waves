use crate::config::{
    ValidationError, load_scenario_config_with_scenarios_dir, registered_action_resolvers,
    registered_event_resolvers, validate_config,
};
use crate::core::{ExternalDecisionInput, RuntimeSession, SessionOptions, WorldState};
use crate::persistence::{ReplaySummary, SqliteStore, replay_summary};
use crate::tui::run_tui_remote_with_hint_and_scenarios_dir;
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct HeadlessReport {
    pub run_id: String,
    pub final_state: WorldState,
    pub decisions: usize,
    pub logs: usize,
    pub domain_events: usize,
    pub ui_events: usize,
    pub pending_decision: bool,
    pub persisted_counts: Option<(u64, u64, u64)>,
    pub persisted_ui_events: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigInspection {
    pub scenario_id: String,
    pub version: String,
    pub default_locale: String,
    pub config_hash: String,
    pub stats: usize,
    pub resources: usize,
    pub actions: usize,
    pub enabled_actions: usize,
    pub events: usize,
    pub panels: usize,
    pub prompts: usize,
    pub balance_keys: usize,
    pub locales: Vec<(String, usize)>,
    pub registered_action_resolvers: usize,
    pub registered_event_resolvers: usize,
}

impl ConfigInspection {
    pub fn lines(&self) -> Vec<String> {
        let locales = self
            .locales
            .iter()
            .map(|(locale, count)| format!("{locale}={count}"))
            .collect::<Vec<_>>()
            .join(" ");
        vec![
            format!("scenario: {}", self.scenario_id),
            format!("version: {}", self.version),
            format!("default_locale: {}", self.default_locale),
            format!("config_hash: {}", self.config_hash),
            format!(
                "tables: stats={} resources={} actions={} enabled_actions={} events={} panels={} prompts={} balance_keys={}",
                self.stats,
                self.resources,
                self.actions,
                self.enabled_actions,
                self.events,
                self.panels,
                self.prompts,
                self.balance_keys,
            ),
            format!("locales: {locales}"),
            format!(
                "resolvers: actions={} events={}",
                self.registered_action_resolvers, self.registered_event_resolvers
            ),
        ]
    }
}

pub fn validate_scenario(scenario_id: &str) -> Result<Vec<ValidationError>> {
    validate_scenario_with_scenarios_dir(scenario_id, None)
}

pub fn validate_scenario_with_scenarios_dir(
    scenario_id: &str,
    scenarios_dir: Option<&Path>,
) -> Result<Vec<ValidationError>> {
    let config = load_scenario_config_with_scenarios_dir(scenario_id, scenarios_dir)?;
    Ok(validate_config(&config))
}

pub fn inspect_config(scenario_id: &str) -> Result<ConfigInspection> {
    inspect_config_with_scenarios_dir(scenario_id, None)
}

pub fn inspect_config_with_scenarios_dir(
    scenario_id: &str,
    scenarios_dir: Option<&Path>,
) -> Result<ConfigInspection> {
    let config = load_scenario_config_with_scenarios_dir(scenario_id, scenarios_dir)?;
    let mut locales = config
        .locales
        .iter()
        .map(|(locale, table)| (locale.clone(), table.entries.len()))
        .collect::<Vec<_>>();
    locales.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(ConfigInspection {
        scenario_id: config.manifest.id,
        version: config.manifest.version,
        default_locale: config.manifest.default_locale,
        config_hash: config.config_hash,
        stats: config.tables.stats.len(),
        resources: config.tables.resources.len(),
        actions: config.tables.actions.len(),
        enabled_actions: config
            .tables
            .actions
            .iter()
            .filter(|action| action.enabled)
            .count(),
        events: config.tables.events.len(),
        panels: config.tables.panels.len(),
        prompts: config.tables.prompts.len(),
        balance_keys: config.tables.balance.len(),
        locales,
        registered_action_resolvers: registered_action_resolvers().len(),
        registered_event_resolvers: registered_event_resolvers().len(),
    })
}

pub fn build_session(
    scenario_id: &str,
    locale: &str,
    seed: u64,
    db_path: Option<PathBuf>,
) -> Result<RuntimeSession> {
    build_session_with_scenarios_dir(scenario_id, locale, seed, db_path, None)
}

pub fn build_session_with_scenarios_dir(
    scenario_id: &str,
    locale: &str,
    seed: u64,
    db_path: Option<PathBuf>,
    scenarios_dir: Option<&Path>,
) -> Result<RuntimeSession> {
    let config = load_scenario_config_with_scenarios_dir(scenario_id, scenarios_dir)?;
    let errors = validate_config(&config);
    if !errors.is_empty() {
        for error in &errors {
            eprintln!("{error}");
        }
        bail!("scenario {scenario_id} failed validation");
    }
    RuntimeSession::new(
        config,
        SessionOptions {
            seed,
            active_locale: locale.to_string(),
            db_path,
            model: "external-agent".to_string(),
        },
    )
}

pub fn build_external_session(
    scenario_id: &str,
    locale: &str,
    seed: u64,
    db_path: Option<PathBuf>,
) -> Result<RuntimeSession> {
    build_session(scenario_id, locale, seed, db_path)
}

pub fn run_play(
    scenario: &str,
    locale: &str,
    seed: u64,
    db_path: Option<PathBuf>,
    socket_path: PathBuf,
    scenarios_dir: Option<PathBuf>,
) -> Result<()> {
    let abs_socket = absolute_socket_path(&socket_path)?;
    let hint = play_connection_hint(&abs_socket);

    let (startup_tx, startup_rx) = mpsc::channel();
    let server_socket = abs_socket.clone();
    let server_scenarios_dir = scenarios_dir.clone();
    let s = scenario.to_string();
    let l = locale.to_string();
    std::thread::spawn(move || {
        let _ = crate::daemon::run_server_with_startup_status_and_scenarios_dir(
            &s,
            &l,
            seed,
            db_path,
            server_socket,
            startup_tx,
            server_scenarios_dir,
        );
    });

    wait_for_daemon_start(&startup_rx, &abs_socket, Duration::from_secs(10))?;

    run_tui_remote_with_hint_and_scenarios_dir(
        abs_socket,
        Duration::from_millis(800),
        hint,
        scenarios_dir,
    )
}

fn absolute_socket_path(socket_path: &Path) -> Result<PathBuf> {
    std::path::absolute(socket_path)
        .with_context(|| format!("resolving socket path {}", socket_path.display()))
}

fn play_connection_hint(abs_socket: &Path) -> String {
    abs_socket.display().to_string()
}

fn wait_for_daemon_start(
    startup_rx: &Receiver<Result<(), String>>,
    socket_path: &Path,
    timeout: Duration,
) -> Result<()> {
    match startup_rx.recv_timeout(timeout) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => bail!("daemon failed to start: {error}"),
        Err(RecvTimeoutError::Timeout) => bail!(
            "daemon did not start within {} seconds (socket: {})",
            timeout.as_secs(),
            socket_path.display()
        ),
        Err(RecvTimeoutError::Disconnected) => bail!(
            "daemon exited before reporting startup status (socket: {})",
            socket_path.display()
        ),
    }
}

pub fn run_headless(
    scenario_id: &str,
    locale: &str,
    ticks: u64,
    seed: u64,
    db_path: Option<PathBuf>,
) -> Result<HeadlessReport> {
    run_headless_with_scenarios_dir(scenario_id, locale, ticks, seed, db_path, None)
}

pub fn run_headless_with_scenarios_dir(
    scenario_id: &str,
    locale: &str,
    ticks: u64,
    seed: u64,
    db_path: Option<PathBuf>,
    scenarios_dir: Option<&Path>,
) -> Result<HeadlessReport> {
    let mut session =
        build_session_with_scenarios_dir(scenario_id, locale, seed, db_path, scenarios_dir)?;
    run_scripted_ticks(&mut session, ticks)?;
    session.save_snapshot_now()?;
    Ok(HeadlessReport {
        run_id: session.run_id.clone(),
        final_state: session.state.clone(),
        decisions: session.decisions.len(),
        logs: session.logs.len(),
        domain_events: session.domain_events.len(),
        ui_events: session.ui_events.len(),
        pending_decision: session.pending_decision().is_some(),
        persisted_counts: session.store_counts()?,
        persisted_ui_events: session.store_ui_event_count()?,
    })
}

pub fn run_scripted_ticks(session: &mut RuntimeSession, ticks: u64) -> Result<()> {
    while session.state.tick < ticks && session.state.outcome.is_none() {
        session.run_ticks(ticks - session.state.tick)?;
        if session.pending_decision().is_some() {
            submit_scripted_decision(session)?;
        }
    }
    Ok(())
}

fn submit_scripted_decision(session: &mut RuntimeSession) -> Result<()> {
    let pending = session
        .pending_decision()
        .ok_or_else(|| anyhow::anyhow!("no pending decision for scripted run"))?
        .clone();
    let action = pending
        .actions
        .first()
        .ok_or_else(|| anyhow::anyhow!("pending decision has no available actions"))?;
    session.submit_external_decision(ExternalDecisionInput {
        decision_id: pending.id,
        action_id: action.id.clone(),
        reason: session.catalog.text("scripted.reason.first_available"),
        risk_attitude: Some("scripted".to_string()),
    })?;
    Ok(())
}

pub fn replay_run(db_path: PathBuf, run_id: &str) -> Result<ReplaySummary> {
    let store = SqliteStore::open(db_path)?;
    replay_summary(&store, run_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn play_connection_hint_uses_absolute_socket_path() {
        let socket =
            absolute_socket_path(Path::new("data/waves.sock")).expect("socket path should resolve");
        let hint = play_connection_hint(&socket);

        assert!(Path::new(&hint).is_absolute());
        assert!(hint.ends_with("data/waves.sock"));
    }

    #[test]
    fn wait_for_daemon_start_returns_reported_error_without_timeout() {
        let (startup_tx, startup_rx) = mpsc::channel();
        startup_tx
            .send(Err("scenario load failed".to_string()))
            .expect("startup status should send");

        let started = Instant::now();
        let error = wait_for_daemon_start(
            &startup_rx,
            Path::new("/tmp/waves-test.sock"),
            Duration::from_secs(10),
        )
        .expect_err("startup error should be returned");

        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(format!("{error:#}").contains("scenario load failed"));
    }
}
