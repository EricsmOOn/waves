use crate::core::{DecisionRecord, DomainEvent, LogEntry, WorldState};
use crate::tui::UiEvent;
use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::json;
use std::path::Path;

pub struct SqliteStore {
    conn: Connection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRecord {
    pub id: String,
    pub scenario_id: String,
    pub scenario_version: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub status: String,
    pub seed: String,
    pub model: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunCounts {
    pub domain_events: u64,
    pub decisions: u64,
    pub logs: u64,
    pub snapshots: u64,
    pub ui_events: u64,
}

impl SqliteStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if path != Path::new(":memory:")
            && let Some(parent) = path.parent()
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating data dir {}", parent.display()))?;
        }
        let conn = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn start_run(
        &self,
        run_id: &str,
        scenario_id: &str,
        scenario_version: &str,
        seed: u64,
        model: &str,
        config_json: serde_json::Value,
    ) -> Result<()> {
        self.conn.execute(
            "insert or ignore into runs
             (id, scenario_id, scenario_version, started_at, status, seed, model, config_json)
             values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                run_id,
                scenario_id,
                scenario_version,
                now_string(),
                "running",
                seed.to_string(),
                model,
                config_json.to_string()
            ],
        )?;
        Ok(())
    }

    pub fn finish_run(&self, run_id: &str, status: &str) -> Result<()> {
        self.conn.execute(
            "update runs set status = ?2, ended_at = ?3 where id = ?1",
            params![run_id, status, now_string()],
        )?;
        Ok(())
    }

    pub fn append_domain_event(&self, run_id: &str, event: &DomainEvent) -> Result<()> {
        self.conn.execute(
            "insert into domain_events
             (event_id, run_id, tick, event_type, created_at, payload_json)
             values (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                event.id,
                run_id,
                event.tick.to_string(),
                event.event_type,
                now_string(),
                event.payload.to_string()
            ],
        )?;
        Ok(())
    }

    pub fn append_decision(&self, run_id: &str, record: &DecisionRecord) -> Result<()> {
        self.conn.execute(
            "insert into decisions
             (run_id, tick, event_id, prompt_json, raw_output, parsed_json, choice, reason,
              risk_attitude, source, parse_status, error, created_at)
             values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                run_id,
                record.tick.to_string(),
                record.event_id,
                json!({}).to_string(),
                record.raw_output,
                serde_json::to_string(record)?,
                record.action_id,
                record.reason,
                record.risk_attitude,
                record.source.to_string(),
                record.parse_status,
                record.error,
                now_string()
            ],
        )?;
        Ok(())
    }

    pub fn append_log(&self, run_id: &str, log: &LogEntry) -> Result<()> {
        self.conn.execute(
            "insert into logs
             (run_id, tick, level, title, body, created_at, source_event_id)
             values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                run_id,
                log.tick.to_string(),
                log.level,
                log.title,
                log.body,
                now_string(),
                log.source_event_id
            ],
        )?;
        Ok(())
    }

    pub fn append_ui_event(&self, run_id: &str, event: &UiEvent) -> Result<()> {
        self.conn.execute(
            "insert into ui_events
             (run_id, tick, ui_event_type, target, payload_json, created_at)
             values (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                run_id,
                event.tick.to_string(),
                event.kind.as_str(),
                event.target,
                serde_json::to_string(event)?,
                now_string()
            ],
        )?;
        Ok(())
    }

    pub fn save_snapshot(&self, run_id: &str, tick: u64, state: &WorldState) -> Result<()> {
        self.conn.execute(
            "insert into snapshots (run_id, tick, created_at, state_json, memory_json)
             values (?1, ?2, ?3, ?4, ?5)",
            params![
                run_id,
                tick.to_string(),
                now_string(),
                serde_json::to_string(state)?,
                serde_json::to_string(&state.memory)?
            ],
        )?;
        Ok(())
    }

    pub fn latest_snapshot(&self, run_id: &str) -> Result<Option<WorldState>> {
        let state_json: Option<String> = self
            .conn
            .query_row(
                "select state_json from snapshots where run_id = ?1 order by cast(tick as integer) desc, id desc limit 1",
                params![run_id],
                |row| row.get(0),
            )
            .optional()?;
        state_json
            .map(|json| serde_json::from_str(&json).context("parsing snapshot state_json"))
            .transpose()
    }

    pub fn run_record(&self, run_id: &str) -> Result<Option<RunRecord>> {
        self.conn
            .query_row(
                "select id, scenario_id, scenario_version, started_at, ended_at, status, seed, model
                 from runs where id = ?1",
                params![run_id],
                |row| {
                    Ok(RunRecord {
                        id: row.get(0)?,
                        scenario_id: row.get(1)?,
                        scenario_version: row.get(2)?,
                        started_at: row.get(3)?,
                        ended_at: row.get(4)?,
                        status: row.get(5)?,
                        seed: row.get(6)?,
                        model: row.get(7)?,
                    })
                },
            )
            .optional()
            .context("querying run record")
    }

    pub fn domain_event_count(&self, run_id: &str) -> Result<u64> {
        count_by_run(&self.conn, "domain_events", run_id)
    }

    pub fn decision_count(&self, run_id: &str) -> Result<u64> {
        count_by_run(&self.conn, "decisions", run_id)
    }

    pub fn log_count(&self, run_id: &str) -> Result<u64> {
        count_by_run(&self.conn, "logs", run_id)
    }

    pub fn ui_event_count(&self, run_id: &str) -> Result<u64> {
        count_by_run(&self.conn, "ui_events", run_id)
    }

    pub fn snapshot_count(&self, run_id: &str) -> Result<u64> {
        count_by_run(&self.conn, "snapshots", run_id)
    }

    pub fn run_counts(&self, run_id: &str) -> Result<RunCounts> {
        Ok(RunCounts {
            domain_events: self.domain_event_count(run_id)?,
            decisions: self.decision_count(run_id)?,
            logs: self.log_count(run_id)?,
            snapshots: self.snapshot_count(run_id)?,
            ui_events: self.ui_event_count(run_id)?,
        })
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            create table if not exists runs (
                id text primary key,
                scenario_id text not null,
                scenario_version text not null,
                started_at text not null,
                ended_at text,
                status text not null,
                seed text not null,
                model text not null,
                config_json text not null
            );

            create table if not exists scenario_versions (
                id integer primary key autoincrement,
                scenario_id text not null,
                version text not null,
                config_hash text not null,
                created_at text not null
            );

            create table if not exists snapshots (
                id integer primary key autoincrement,
                run_id text not null,
                tick text not null,
                created_at text not null,
                state_json text not null,
                memory_json text not null,
                foreign key(run_id) references runs(id)
            );

            create table if not exists domain_events (
                id integer primary key autoincrement,
                event_id text not null,
                run_id text not null,
                tick text not null,
                event_type text not null,
                created_at text not null,
                payload_json text not null,
                foreign key(run_id) references runs(id)
            );

            create table if not exists decisions (
                id integer primary key autoincrement,
                run_id text not null,
                tick text not null,
                event_id text not null,
                prompt_json text not null,
                raw_output text,
                parsed_json text not null,
                choice text not null,
                reason text not null,
                risk_attitude text not null,
                source text not null,
                parse_status text not null,
                error text,
                created_at text not null,
                foreign key(run_id) references runs(id)
            );

            create table if not exists logs (
                id integer primary key autoincrement,
                run_id text not null,
                tick text not null,
                level text not null,
                title text not null,
                body text not null,
                created_at text not null,
                source_event_id text,
                foreign key(run_id) references runs(id)
            );

            create table if not exists ui_events (
                id integer primary key autoincrement,
                run_id text not null,
                tick text not null,
                ui_event_type text not null,
                target text not null,
                payload_json text not null,
                created_at text not null,
                foreign key(run_id) references runs(id)
            );
            "#,
        )?;
        Ok(())
    }
}

fn count_by_run(conn: &Connection, table: &str, run_id: &str) -> Result<u64> {
    let sql = format!("select count(*) from {table} where run_id = ?1");
    let count: i64 = conn.query_row(&sql, params![run_id], |row| row.get(0))?;
    Ok(count as u64)
}

fn now_string() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string())
}
