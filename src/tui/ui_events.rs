use crate::core::{DecisionSource, LogEntry, Resolution, RiskLevel};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UiEventKind {
    ValueDelta,
    Risk,
    Decision,
    Fallback,
    Log,
    WorldEvent,
    Control,
}

impl UiEventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            UiEventKind::ValueDelta => "value_delta",
            UiEventKind::Risk => "risk",
            UiEventKind::Decision => "decision",
            UiEventKind::Fallback => "fallback",
            UiEventKind::Log => "log",
            UiEventKind::WorldEvent => "world_event",
            UiEventKind::Control => "control",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum UiPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UiMotion {
    None,
    Fade,
    Pulse,
    Rise,
    Sink,
    Slide,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UiVisibility {
    Timed,
    Persistent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiEvent {
    pub id: String,
    pub tick: u64,
    pub created_frame: u64,
    pub ttl_frames: u64,
    pub motion: UiMotion,
    pub visibility: UiVisibility,
    pub kind: UiEventKind,
    pub priority: UiPriority,
    pub target: String,
    pub message: String,
    pub delta: Option<f64>,
    pub value: Option<f64>,
}

impl UiEvent {
    pub fn age(&self, current_frame: u64) -> u64 {
        current_frame.saturating_sub(self.created_frame)
    }

    pub fn is_visible(&self, current_frame: u64) -> bool {
        self.visibility == UiVisibility::Persistent || self.age(current_frame) <= self.ttl_frames
    }

    pub fn intensity(&self, current_frame: u64) -> f64 {
        if self.visibility == UiVisibility::Persistent {
            return 1.0;
        }
        if self.ttl_frames == 0 {
            return 0.0;
        }
        let age = self.age(current_frame).min(self.ttl_frames) as f64;
        1.0 - (age / self.ttl_frames as f64)
    }

    pub fn with_created_frame(mut self, frame: u64) -> Self {
        self.created_frame = frame;
        self
    }

    pub fn world_event(tick: u64, target: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(
            tick,
            UiEventKind::WorldEvent,
            UiPriority::High,
            target,
            message,
            None,
            None,
        )
    }

    pub fn control(tick: u64, target: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(
            tick,
            UiEventKind::Control,
            UiPriority::Normal,
            target,
            message,
            None,
            None,
        )
    }

    pub fn decision(tick: u64, action_id: impl Into<String>, source: &DecisionSource) -> Self {
        let action_id = action_id.into();
        let (kind, priority, message) = if *source == DecisionSource::Fallback {
            (UiEventKind::Fallback, UiPriority::High, action_id.clone())
        } else {
            (UiEventKind::Decision, UiPriority::Normal, action_id.clone())
        };
        Self::new(tick, kind, priority, action_id, message, None, None)
    }

    pub fn risk(tick: u64, risk: RiskLevel) -> Self {
        let priority = match risk {
            RiskLevel::Low => UiPriority::Low,
            RiskLevel::Medium => UiPriority::Normal,
            RiskLevel::High => UiPriority::High,
            RiskLevel::Critical => UiPriority::Critical,
        };
        Self::new(
            tick,
            UiEventKind::Risk,
            priority,
            risk.key(),
            risk.key(),
            None,
            None,
        )
    }

    pub fn log(log: &LogEntry) -> Self {
        let priority = match log.level.as_str() {
            "danger" | "critical" => UiPriority::Critical,
            "warning" => UiPriority::High,
            "notice" => UiPriority::Normal,
            _ => UiPriority::Low,
        };
        Self::new(
            log.tick,
            UiEventKind::Log,
            priority,
            log.title.clone(),
            log.body.clone(),
            None,
            None,
        )
    }

    pub fn from_resolution(tick: u64, resolution: &Resolution) -> Vec<Self> {
        resolution
            .changes
            .iter()
            .map(|change| {
                let priority =
                    if matches!(change.target.as_str(), "hp" | "raft") && change.delta < -5.0 {
                        UiPriority::Critical
                    } else if change.delta < 0.0 {
                        UiPriority::High
                    } else if change.delta > 0.0 {
                        UiPriority::Normal
                    } else {
                        UiPriority::Low
                    };
                let sign = if change.delta >= 0.0 { "+" } else { "" };
                Self::new(
                    tick,
                    UiEventKind::ValueDelta,
                    priority,
                    change.target.clone(),
                    format!("{} {}{:.2}", change.target, sign, change.delta),
                    Some(change.delta),
                    Some(change.after),
                )
            })
            .collect()
    }

    fn new(
        tick: u64,
        kind: UiEventKind,
        priority: UiPriority,
        target: impl Into<String>,
        message: impl Into<String>,
        delta: Option<f64>,
        value: Option<f64>,
    ) -> Self {
        let ttl_frames = default_ttl_frames(&kind, &priority);
        let motion = default_motion(&kind, delta);
        Self {
            id: Uuid::new_v4().to_string(),
            tick,
            created_frame: tick,
            ttl_frames,
            motion,
            visibility: UiVisibility::Timed,
            kind,
            priority,
            target: target.into(),
            message: message.into(),
            delta,
            value,
        }
    }
}

fn default_ttl_frames(kind: &UiEventKind, priority: &UiPriority) -> u64 {
    let base = match kind {
        UiEventKind::ValueDelta => 24,
        UiEventKind::Risk => 30,
        UiEventKind::Decision => 18,
        UiEventKind::Fallback => 36,
        UiEventKind::Log => 18,
        UiEventKind::WorldEvent => 30,
        UiEventKind::Control => 20,
    };
    base + match priority {
        UiPriority::Low => 0,
        UiPriority::Normal => 4,
        UiPriority::High => 10,
        UiPriority::Critical => 18,
    }
}

fn default_motion(kind: &UiEventKind, delta: Option<f64>) -> UiMotion {
    match kind {
        UiEventKind::ValueDelta if delta.unwrap_or_default() > 0.0 => UiMotion::Rise,
        UiEventKind::ValueDelta if delta.unwrap_or_default() < 0.0 => UiMotion::Sink,
        UiEventKind::Risk | UiEventKind::Fallback => UiMotion::Pulse,
        UiEventKind::WorldEvent | UiEventKind::Decision | UiEventKind::Control => UiMotion::Slide,
        UiEventKind::Log | UiEventKind::ValueDelta => UiMotion::Fade,
    }
}
