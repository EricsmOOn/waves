use crate::config::LoadedScenarioConfig;
use crate::core::{
    AiDecision, DecisionRecord, DecisionSource, DomainEvent, LogEntry, RiskLevel, SeaCondition,
    Weather, WorldEvent, WorldState,
};
use crate::core::{ExternalDecisionInput, PendingDecision};
use crate::i18n::Catalog;
use crate::persistence::SqliteStore;
use crate::scenario::{Scenario, build_scenario};
use crate::tui::UiEvent;
use anyhow::{Result, anyhow};
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde_json::json;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SessionOptions {
    pub seed: u64,
    pub active_locale: String,
    pub db_path: Option<PathBuf>,
    pub model: String,
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self {
            seed: 42,
            active_locale: "zh-CN".to_string(),
            db_path: None,
            model: "external-agent".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct StepReport {
    pub events: Vec<DomainEvent>,
    pub decisions: Vec<DecisionRecord>,
    pub logs: Vec<LogEntry>,
    pub ui_events: Vec<UiEvent>,
    pub snapshot_saved: bool,
}

pub struct RuntimeSession {
    pub run_id: String,
    pub config: LoadedScenarioConfig,
    pub catalog: Catalog,
    pub state: WorldState,
    pub logs: Vec<LogEntry>,
    pub decisions: Vec<DecisionRecord>,
    pub domain_events: Vec<DomainEvent>,
    pub ui_events: Vec<UiEvent>,
    pub paused: bool,
    pub presentation_frame: u64,
    pending_decision: Option<PendingDecision>,
    scenario: Box<dyn Scenario>,
    rng: StdRng,
    store: Option<SqliteStore>,
    event_interval_ticks: u64,
    snapshot_interval_ticks: u64,
}

impl RuntimeSession {
    pub fn new(config: LoadedScenarioConfig, options: SessionOptions) -> Result<Self> {
        let scenario = build_scenario(config.clone())?;
        let catalog = Catalog::new(
            options.active_locale.clone(),
            config.manifest.default_locale.clone(),
            config.locales.clone(),
        );
        let state = scenario.initial_state();
        let run_id = Uuid::new_v4().to_string();
        let store = match &options.db_path {
            Some(path) => Some(SqliteStore::open(path)?),
            None => None,
        };
        if let Some(store) = &store {
            store.start_run(
                &run_id,
                &config.manifest.id,
                &config.manifest.version,
                options.seed,
                &options.model,
                json!({
                    "scenario_id": config.manifest.id,
                    "scenario_version": config.manifest.version,
                    "default_locale": config.manifest.default_locale,
                    "active_locale": options.active_locale,
                    "config_hash": config.config_hash,
                }),
            )?;
            store.save_snapshot(&run_id, 0, &state)?;
        }

        let event_interval_ticks = config
            .tables
            .balance
            .get("event_interval_ticks")
            .copied()
            .unwrap_or(4.0)
            .max(1.0) as u64;
        let snapshot_interval_ticks = config
            .tables
            .balance
            .get("snapshot_interval_ticks")
            .copied()
            .unwrap_or(6.0)
            .max(1.0) as u64;

        Ok(Self {
            run_id,
            config,
            catalog,
            state,
            logs: Vec::new(),
            decisions: Vec::new(),
            domain_events: Vec::new(),
            ui_events: Vec::new(),
            paused: false,
            presentation_frame: 0,
            pending_decision: None,
            scenario,
            rng: StdRng::seed_from_u64(options.seed),
            store,
            event_interval_ticks,
            snapshot_interval_ticks,
        })
    }

    pub fn step(&mut self) -> Result<StepReport> {
        if self.paused || self.state.outcome.is_some() || self.pending_decision.is_some() {
            return Ok(StepReport::default());
        }

        let mut report = StepReport::default();
        let tick_events = self.scenario.apply_tick(&mut self.state);
        for event in tick_events {
            self.record_domain_event(event.clone())?;
            report.events.push(event);
        }

        if self.state.tick.is_multiple_of(self.event_interval_ticks) && self.state.outcome.is_none()
        {
            let world_event = self.scenario.select_event(&self.state, &mut self.rng);
            apply_world_event_effect(&mut self.state, &world_event);
            let event_record = DomainEvent::new(
                self.state.tick,
                "world_event",
                json!({
                    "event": world_event,
                    "state_after_event": self.state,
                }),
            );
            self.record_domain_event(event_record.clone())?;
            report.events.push(event_record.clone());
            let ui_event = UiEvent::world_event(
                self.state.tick,
                world_event.id.clone(),
                self.catalog.text(&world_event.title_key),
            );
            self.record_ui_event(ui_event.clone())?;
            report.ui_events.push(ui_event);

            let actions = self.scenario.available_actions(&self.state, &world_event);
            if actions.is_empty() {
                return Err(anyhow!("scenario returned no available actions"));
            }
            self.pending_decision = Some(PendingDecision {
                id: pending_decision_id(self.state.tick, &world_event),
                tick: self.state.tick,
                scenario_id: self.config.manifest.id.clone(),
                event: world_event,
                actions,
                state: self.state.clone(),
            });
            return Ok(report);
        }

        self.maybe_save_snapshot(&mut report)?;
        Ok(report)
    }

    pub fn advance_presentation_frame(&mut self) {
        self.presentation_frame = self.presentation_frame.saturating_add(1);
    }

    pub fn toggle_pause(&mut self) -> Result<()> {
        self.paused = !self.paused;
        let message = if self.paused {
            self.catalog.text("control.paused")
        } else {
            self.catalog.text("control.resumed")
        };
        self.record_intervention("pause", &message)
    }

    pub fn run_ticks(&mut self, ticks: u64) -> Result<()> {
        for _ in 0..ticks {
            if self.state.outcome.is_some() || self.pending_decision.is_some() {
                break;
            }
            self.step()?;
        }
        Ok(())
    }

    pub fn pending_decision(&self) -> Option<&PendingDecision> {
        self.pending_decision.as_ref()
    }

    pub fn submit_external_decision(&mut self, input: ExternalDecisionInput) -> Result<StepReport> {
        if self.state.outcome.is_some() {
            return Err(anyhow!("run already finished"));
        }
        let pending = self
            .pending_decision
            .as_ref()
            .ok_or_else(|| anyhow!("no pending decision"))?;
        if pending.id != input.decision_id {
            return Err(anyhow!("stale decision id {}", input.decision_id));
        }
        if input.reason.trim().is_empty() {
            return Err(anyhow!("decision reason is required"));
        }
        if !pending
            .actions
            .iter()
            .any(|action| action.id == input.action_id)
        {
            return Err(anyhow!("action {:?} is not available", input.action_id));
        }

        let pending = self
            .pending_decision
            .take()
            .expect("pending decision checked above");
        let mut report = StepReport::default();
        let decision = AiDecision {
            choice: input.action_id,
            reason: input.reason.trim().to_string(),
            risk_attitude: input
                .risk_attitude
                .filter(|attitude| !attitude.trim().is_empty())
                .unwrap_or_else(|| "agent".to_string()),
            source: DecisionSource::Agent,
            raw_output: None,
        };
        self.resolve_decision(
            &pending.event,
            pending.id,
            decision,
            "ok".to_string(),
            None,
            &mut report,
        )?;
        self.maybe_save_snapshot(&mut report)?;
        Ok(report)
    }

    pub fn store_counts(&self) -> Result<Option<(u64, u64, u64)>> {
        match &self.store {
            Some(store) => Ok(Some((
                store.domain_event_count(&self.run_id)?,
                store.decision_count(&self.run_id)?,
                store.log_count(&self.run_id)?,
            ))),
            None => Ok(None),
        }
    }

    pub fn store_ui_event_count(&self) -> Result<Option<u64>> {
        match &self.store {
            Some(store) => Ok(Some(store.ui_event_count(&self.run_id)?)),
            None => Ok(None),
        }
    }

    pub fn latest_persisted_snapshot(&self) -> Result<Option<WorldState>> {
        match &self.store {
            Some(store) => store.latest_snapshot(&self.run_id),
            None => Ok(None),
        }
    }

    pub fn save_snapshot_now(&self) -> Result<()> {
        if let Some(store) = &self.store {
            store.save_snapshot(&self.run_id, self.state.tick, &self.state)?;
        }
        Ok(())
    }

    fn resolve_decision(
        &mut self,
        world_event: &WorldEvent,
        source_event_id: String,
        decision: AiDecision,
        parse_status: String,
        error: Option<String>,
        report: &mut StepReport,
    ) -> Result<()> {
        let decision_record = DecisionRecord {
            tick: self.state.tick,
            event_id: world_event.id.clone(),
            action_id: decision.choice.clone(),
            reason: decision.reason.clone(),
            risk_attitude: decision.risk_attitude.clone(),
            source: decision.source.clone(),
            parse_status,
            raw_output: decision.raw_output.clone(),
            error,
        };
        self.record_decision(decision_record.clone())?;
        report.decisions.push(decision_record);
        let ui_event =
            UiEvent::decision(self.state.tick, decision.choice.clone(), &decision.source);
        self.record_ui_event(ui_event.clone())?;
        report.ui_events.push(ui_event);

        let resolution = self.scenario.resolve_action(
            &mut self.state,
            world_event,
            &decision.choice,
            &self.catalog,
            &mut self.rng,
        );
        let resolution_event = DomainEvent::new(
            self.state.tick,
            "resolution",
            json!({
                "event": world_event,
                "decision": decision,
                "resolution": resolution,
                "state_after_resolution": self.state,
            }),
        );
        self.record_domain_event(resolution_event.clone())?;
        report.events.push(resolution_event.clone());
        for ui_event in UiEvent::from_resolution(self.state.tick, &resolution) {
            self.record_ui_event(ui_event.clone())?;
            report.ui_events.push(ui_event);
        }
        let risk_ui_event = UiEvent::risk(self.state.tick, self.state.environment.risk);
        self.record_ui_event(risk_ui_event.clone())?;
        report.ui_events.push(risk_ui_event);

        let mut logs = self.logs_for_resolution(world_event, &decision, &resolution.summary);
        if decision.source == DecisionSource::Fallback {
            logs.insert(
                0,
                LogEntry {
                    tick: self.state.tick,
                    level: "warning".to_string(),
                    title: self.catalog.text("source.fallback"),
                    body: self.catalog.format(
                        "log.fallback",
                        &[("action", self.action_name(&decision.choice))],
                    ),
                    source_event_id: Some(source_event_id),
                },
            );
        }
        for log in logs {
            self.record_log(log.clone())?;
            let ui_event = UiEvent::log(&log);
            self.record_ui_event(ui_event.clone())?;
            report.ui_events.push(ui_event);
            report.logs.push(log);
        }
        Ok(())
    }

    fn maybe_save_snapshot(&self, report: &mut StepReport) -> Result<()> {
        if (self.state.tick.is_multiple_of(self.snapshot_interval_ticks)
            || self.state.outcome.is_some())
            && let Some(store) = &self.store
        {
            store.save_snapshot(&self.run_id, self.state.tick, &self.state)?;
            report.snapshot_saved = true;
        }
        if let Some(outcome) = &self.state.outcome
            && let Some(store) = &self.store
        {
            store.finish_run(&self.run_id, outcome)?;
        }
        Ok(())
    }

    fn logs_for_resolution(
        &self,
        event: &WorldEvent,
        decision: &AiDecision,
        summary: &str,
    ) -> Vec<LogEntry> {
        let event_title = self.catalog.text(&event.title_key);
        let action_name = self.action_name(&decision.choice);
        vec![
            LogEntry {
                tick: self.state.tick,
                level: event.severity.clone(),
                title: event_title.clone(),
                body: self.catalog.format(
                    "log.decision",
                    &[
                        ("actor", self.catalog.text("source.agent")),
                        ("action", action_name.clone()),
                        ("reason", decision.reason.clone()),
                    ],
                ),
                source_event_id: Some(event.id.clone()),
            },
            LogEntry {
                tick: self.state.tick,
                level: if self.state.environment.risk == RiskLevel::High {
                    "warning".to_string()
                } else {
                    "notice".to_string()
                },
                title: action_name.clone(),
                body: self.catalog.format(
                    "log.result",
                    &[("action", action_name), ("summary", summary.to_string())],
                ),
                source_event_id: Some(event.id.clone()),
            },
        ]
    }

    fn action_name(&self, action_id: &str) -> String {
        self.config
            .tables
            .actions
            .iter()
            .find(|action| action.id == action_id)
            .map(|action| self.catalog.text(&action.name_key))
            .unwrap_or_else(|| action_id.to_string())
    }

    fn record_domain_event(&mut self, event: DomainEvent) -> Result<()> {
        if let Some(store) = &self.store {
            store.append_domain_event(&self.run_id, &event)?;
        }
        self.domain_events.push(event);
        Ok(())
    }

    fn record_decision(&mut self, decision: DecisionRecord) -> Result<()> {
        if let Some(store) = &self.store {
            store.append_decision(&self.run_id, &decision)?;
        }
        self.decisions.push(decision);
        Ok(())
    }

    fn record_log(&mut self, log: LogEntry) -> Result<()> {
        if let Some(store) = &self.store {
            store.append_log(&self.run_id, &log)?;
        }
        self.logs.push(log);
        if self.logs.len() > 100 {
            self.logs.remove(0);
        }
        Ok(())
    }

    fn record_ui_event(&mut self, event: UiEvent) -> Result<()> {
        let event = event.with_created_frame(self.presentation_frame);
        if let Some(store) = &self.store {
            store.append_ui_event(&self.run_id, &event)?;
        }
        self.ui_events.push(event);
        if self.ui_events.len() > 120 {
            self.ui_events.remove(0);
        }
        Ok(())
    }

    fn record_intervention(&mut self, target: &str, message: &str) -> Result<()> {
        let log = LogEntry {
            tick: self.state.tick,
            level: "notice".to_string(),
            title: target.to_string(),
            body: message.to_string(),
            source_event_id: None,
        };
        self.record_log(log)?;
        self.record_ui_event(UiEvent::control(self.state.tick, target, message))?;
        Ok(())
    }
}

fn pending_decision_id(tick: u64, event: &WorldEvent) -> String {
    format!("tick-{tick}-{}", event.id)
}

fn apply_world_event_effect(state: &mut WorldState, event: &WorldEvent) {
    match event.id.as_str() {
        "rain" => {
            state.environment.weather = Weather::Rain;
            state.environment.sea = SeaCondition::Moderate;
        }
        "heat" => {
            state.environment.weather = Weather::Heat;
            state.stats.thirst = (state.stats.thirst + 3.0).clamp(0.0, 100.0);
        }
        "storm" => {
            state.environment.weather = Weather::Storm;
            state.environment.sea = SeaCondition::Rough;
            state.environment.risk = RiskLevel::High;
        }
        "hull_damage" => {
            state.stats.raft = (state.stats.raft - 5.0).clamp(0.0, 100.0);
        }
        "fish_shoal" => {
            state.environment.sea = SeaCondition::Calm;
        }
        "island_silhouette" => {
            state.environment.distance_to_land =
                (state.environment.distance_to_land - 5.0).max(0.0);
        }
        _ => {}
    }
}
