use crate::core::WorldState;
use crate::persistence::SqliteStore;
use crate::persistence::sqlite::{RunCounts, RunRecord};
use anyhow::{Result, bail};

#[derive(Debug, Clone, PartialEq)]
pub struct ReplaySummary {
    pub run: RunRecord,
    pub latest_snapshot: Option<WorldState>,
    pub counts: RunCounts,
}

impl ReplaySummary {
    pub fn lines(&self) -> Vec<String> {
        let snapshot_tick = self
            .latest_snapshot
            .as_ref()
            .map(|state| state.tick.to_string())
            .unwrap_or_else(|| "none".to_string());
        let outcome = self
            .latest_snapshot
            .as_ref()
            .and_then(|state| state.outcome.clone())
            .unwrap_or_else(|| "running".to_string());
        vec![
            format!("run_id: {}", self.run.id),
            format!(
                "scenario: {} {}",
                self.run.scenario_id, self.run.scenario_version
            ),
            format!("status: {}", self.run.status),
            format!("seed: {}", self.run.seed),
            format!("model: {}", self.run.model),
            format!("snapshot_tick: {snapshot_tick}"),
            format!("outcome: {outcome}"),
            format!(
                "counts: domain_events={} decisions={} logs={} snapshots={} ui_events={}",
                self.counts.domain_events,
                self.counts.decisions,
                self.counts.logs,
                self.counts.snapshots,
                self.counts.ui_events,
            ),
        ]
    }
}

pub fn load_latest_snapshot(store: &SqliteStore, run_id: &str) -> Result<Option<WorldState>> {
    store.latest_snapshot(run_id)
}

pub fn replay_summary(store: &SqliteStore, run_id: &str) -> Result<ReplaySummary> {
    let Some(run) = store.run_record(run_id)? else {
        bail!("run_id {run_id} was not found");
    };
    Ok(ReplaySummary {
        run,
        latest_snapshot: store.latest_snapshot(run_id)?,
        counts: store.run_counts(run_id)?,
    })
}
