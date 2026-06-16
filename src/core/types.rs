use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Weather {
    Clear,
    Cloudy,
    Rain,
    Heat,
    Storm,
}

impl Weather {
    pub fn key(self) -> &'static str {
        match self {
            Weather::Clear => "weather.clear",
            Weather::Cloudy => "weather.cloudy",
            Weather::Rain => "weather.rain",
            Weather::Heat => "weather.heat",
            Weather::Storm => "weather.storm",
        }
    }
}

impl fmt::Display for Weather {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SeaCondition {
    Calm,
    Moderate,
    Rough,
}

impl SeaCondition {
    pub fn key(self) -> &'static str {
        match self {
            SeaCondition::Calm => "sea.calm",
            SeaCondition::Moderate => "sea.moderate",
            SeaCondition::Rough => "sea.rough",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn key(self) -> &'static str {
        match self {
            RiskLevel::Low => "risk.low",
            RiskLevel::Medium => "risk.medium",
            RiskLevel::High => "risk.high",
            RiskLevel::Critical => "risk.critical",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Stats {
    pub hp: f64,
    pub hunger: f64,
    pub thirst: f64,
    pub energy: f64,
    pub morale: f64,
    pub raft: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Resources {
    pub food: f64,
    pub water: f64,
    pub wood: f64,
    pub fiber: f64,
    pub tool: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Environment {
    pub weather: Weather,
    pub sea: SeaCondition,
    pub wind: String,
    pub risk: RiskLevel,
    pub day: u32,
    pub minute_of_day: u32,
    pub distance_to_land: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Personality {
    pub risk_bias: f64,
    pub water_priority: f64,
    pub exploration_bias: f64,
    pub repair_priority: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Memory {
    pub goal_key: String,
    pub concern_key: String,
    pub recent: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldState {
    pub tick: u64,
    pub stats: Stats,
    pub resources: Resources,
    pub environment: Environment,
    pub personality: Personality,
    pub memory: Memory,
    pub alive: bool,
    pub outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldEvent {
    pub id: String,
    pub title_key: String,
    pub severity: String,
    pub resolver_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionOption {
    pub id: String,
    pub name_key: String,
    pub risk: String,
    pub resolver_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PendingDecision {
    pub id: String,
    pub tick: u64,
    pub scenario_id: String,
    pub event: WorldEvent,
    pub actions: Vec<ActionOption>,
    pub state: WorldState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalDecisionInput {
    pub decision_id: String,
    pub action_id: String,
    pub reason: String,
    pub risk_attitude: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionSource {
    Fallback,
    Agent,
}

impl fmt::Display for DecisionSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecisionSource::Fallback => write!(f, "fallback"),
            DecisionSource::Agent => write!(f, "agent"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiDecision {
    pub choice: String,
    pub reason: String,
    pub risk_attitude: String,
    pub source: DecisionSource,
    pub raw_output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StateChange {
    pub target: String,
    pub before: f64,
    pub after: f64,
    pub delta: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Resolution {
    pub success: bool,
    pub summary: String,
    pub changes: Vec<StateChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainEvent {
    pub id: String,
    pub tick: u64,
    pub event_type: String,
    pub payload: Value,
}

impl DomainEvent {
    pub fn new(tick: u64, event_type: impl Into<String>, payload: Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            tick,
            event_type: event_type.into(),
            payload,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionRecord {
    pub tick: u64,
    pub event_id: String,
    pub action_id: String,
    pub reason: String,
    pub risk_attitude: String,
    pub source: DecisionSource,
    pub parse_status: String,
    pub raw_output: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogEntry {
    pub tick: u64,
    pub level: String,
    pub title: String,
    pub body: String,
    pub source_event_id: Option<String>,
}
