use crate::core::RiskLevel;
use crate::tui::{UiEvent, UiPriority};
use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleRole {
    Title,
    Text,
    Muted,
    Positive,
    Negative,
    Warning,
    Critical,
    Recovery,
}

pub struct Theme;

impl Theme {
    pub fn color(role: StyleRole) -> Color {
        match role {
            StyleRole::Title => Color::Cyan,
            StyleRole::Text => Color::White,
            StyleRole::Muted => Color::DarkGray,
            StyleRole::Positive => Color::Green,
            StyleRole::Negative => Color::Red,
            StyleRole::Warning => Color::Yellow,
            StyleRole::Critical => Color::Red,
            StyleRole::Recovery => Color::Cyan,
        }
    }

    pub fn style(role: StyleRole) -> Style {
        match role {
            StyleRole::Title | StyleRole::Critical => Style::default()
                .fg(Self::color(role))
                .add_modifier(Modifier::BOLD),
            _ => Style::default().fg(Self::color(role)),
        }
    }

    pub fn role_for_delta(delta: f64) -> StyleRole {
        if delta > 0.0 {
            StyleRole::Positive
        } else if delta < 0.0 {
            StyleRole::Negative
        } else {
            StyleRole::Muted
        }
    }

    pub fn role_for_priority(priority: &UiPriority) -> StyleRole {
        match priority {
            UiPriority::Low => StyleRole::Muted,
            UiPriority::Normal => StyleRole::Text,
            UiPriority::High => StyleRole::Warning,
            UiPriority::Critical => StyleRole::Critical,
        }
    }

    pub fn role_for_risk(risk: RiskLevel) -> StyleRole {
        match risk {
            RiskLevel::Low => StyleRole::Recovery,
            RiskLevel::Medium => StyleRole::Warning,
            RiskLevel::High | RiskLevel::Critical => StyleRole::Critical,
        }
    }

    pub fn role_for_event(event: &UiEvent) -> StyleRole {
        if let Some(delta) = event.delta {
            Self::role_for_delta(delta)
        } else {
            Self::role_for_priority(&event.priority)
        }
    }
}
