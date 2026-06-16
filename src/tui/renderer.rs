use crate::core::{DecisionRecord, DecisionSource, LogEntry, PendingDecision, WorldState};
use crate::i18n::Catalog;
use crate::tui::text_width::{Align, pad_to_width, truncate_to_width, wrap_to_width};
use crate::tui::{StyleRole, Theme, UiEvent, UiEventKind};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap};

pub struct AppView<'a> {
    pub run_id: &'a str,
    pub scenario_id: &'a str,
    pub config_hash: &'a str,
    pub state: &'a WorldState,
    pub catalog: &'a Catalog,
    pub logs: &'a [LogEntry],
    pub decisions: &'a [DecisionRecord],
    pub ui_events: &'a [UiEvent],
    pub pending_decision: Option<&'a PendingDecision>,
    pub current_frame: u64,
    pub paused: bool,
}

pub fn render_app(frame: &mut Frame<'_>, view: &AppView<'_>) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(12),
            Constraint::Length(3),
        ])
        .split(frame.area());

    render_header(frame, root[0], view);
    render_body(frame, root[1], view);
    render_footer(frame, root[2], view);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, view: &AppView<'_>) {
    let status_key = if view.paused {
        "status.paused"
    } else {
        "status.running"
    };
    let scenario_title_key = format!("scenario.{}.title", view.scenario_id);
    let title = format!(
        "{} · {} · {} {} · {} {} · {}",
        view.catalog.text("app.title"),
        view.catalog.text(&scenario_title_key),
        view.catalog.text("label.tick"),
        view.state.tick,
        view.catalog.text("label.day"),
        view.state.environment.day,
        view.catalog.text(status_key),
    );
    let paragraph = Paragraph::new(title)
        .style(Theme::style(StyleRole::Title))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}

fn render_body(frame: &mut Frame<'_>, area: Rect, view: &AppView<'_>) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(32),
            Constraint::Percentage(36),
            Constraint::Percentage(32),
        ])
        .split(area);
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(46),
            Constraint::Percentage(28),
            Constraint::Percentage(26),
        ])
        .split(columns[0]);
    render_status(frame, left[0], view);
    render_resources(frame, left[1], view);
    render_environment(frame, left[2], view);

    let middle = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Min(8),
        ])
        .split(columns[1]);
    render_ai(frame, middle[0], view);
    render_activity(frame, middle[1], view);
    render_logs(frame, middle[2], view);
    render_decisions(frame, columns[2], view);
}

fn render_status(frame: &mut Frame<'_>, area: Rect, view: &AppView<'_>) {
    let stats = [
        ("stat.hp", view.state.stats.hp, Color::Green),
        (
            "stat.hunger",
            100.0 - view.state.stats.hunger,
            Color::Yellow,
        ),
        (
            "stat.thirst",
            100.0 - view.state.stats.thirst,
            Color::Yellow,
        ),
        ("stat.energy", view.state.stats.energy, Color::Blue),
        ("stat.morale", view.state.stats.morale, Color::Magenta),
        (
            "stat.raft",
            view.state.stats.raft,
            Theme::color(Theme::role_for_risk(view.state.environment.risk)),
        ),
    ];
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            stats
                .iter()
                .map(|_| Constraint::Length(1))
                .collect::<Vec<_>>(),
        )
        .split(area.inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 1,
        }));
    frame.render_widget(
        Block::default()
            .title(view.catalog.text("panel.status"))
            .borders(Borders::ALL),
        area,
    );
    for (idx, (key, value, color)) in stats.iter().enumerate() {
        let label = truncate_to_width(&view.catalog.text(key), 8);
        let gauge = Gauge::default()
            .label(format!(
                "{} {:>3.0}%",
                pad_to_width(&label, 8, Align::Left),
                value
            ))
            .gauge_style(Style::default().fg(*color))
            .percent(value.round().clamp(0.0, 100.0) as u16);
        frame.render_widget(gauge, rows[idx]);
    }
}

fn render_resources(frame: &mut Frame<'_>, area: Rect, view: &AppView<'_>) {
    let day_unit = view.catalog.text("unit.days_short");
    let lines = vec![
        resource_line(view, "resource.food", view.state.resources.food, &day_unit),
        resource_line(
            view,
            "resource.water",
            view.state.resources.water,
            &day_unit,
        ),
        resource_line(view, "resource.wood", view.state.resources.wood, ""),
        resource_line(view, "resource.fiber", view.state.resources.fiber, ""),
        resource_line(view, "resource.tool", view.state.resources.tool, ""),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title(view.catalog.text("panel.resources"))
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_environment(frame: &mut Frame<'_>, area: Rect, view: &AppView<'_>) {
    let state = view.state;
    let lines = vec![
        environment_line(
            view,
            "label.weather",
            view.catalog.text(state.environment.weather.key()),
        ),
        environment_line(
            view,
            "label.sea",
            view.catalog.text(state.environment.sea.key()),
        ),
        environment_line(
            view,
            "label.risk",
            view.catalog.text(state.environment.risk.key()),
        ),
        environment_line(
            view,
            "nav.target",
            format!(
                "{:.1} {}",
                state.environment.distance_to_land,
                view.catalog.text("unit.distance")
            ),
        ),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(view.catalog.text("panel.environment"))
                .borders(Borders::ALL),
        ),
        area,
    );
}

fn render_ai(frame: &mut Frame<'_>, area: Rect, view: &AppView<'_>) {
    let state = view.state;
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                format!("{} ", view.catalog.text("label.goal")),
                Theme::style(StyleRole::Muted),
            ),
            Span::raw(view.catalog.text(&state.memory.goal_key)),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{} ", view.catalog.text("label.concern")),
                Theme::style(StyleRole::Muted),
            ),
            Span::styled(
                view.catalog.text(&state.memory.concern_key),
                Theme::style(Theme::role_for_risk(state.environment.risk)),
            ),
        ]),
        Line::from(format!(
            "{} {:+.0}",
            pad_to_width(&view.catalog.text("label.risk_bias"), 10, Align::Left),
            state.personality.risk_bias
        )),
        Line::from(format!(
            "{} {:+.0}",
            pad_to_width(&view.catalog.text("label.explore"), 10, Align::Left),
            state.personality.exploration_bias
        )),
    ];
    if let Some(pending) = view.pending_decision {
        let width = area.width.saturating_sub(4) as usize;
        let actions = pending
            .actions
            .iter()
            .map(|action| view.catalog.text(&action.name_key))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(Line::from(vec![
            Span::styled(
                format!("{} ", view.catalog.text("label.agent")),
                Theme::style(StyleRole::Warning),
            ),
            Span::raw(view.catalog.text("label.waiting")),
        ]));
        lines.push(Line::from(truncate_to_width(
            &format!("{} {actions}", view.catalog.text("label.actions")),
            width,
        )));
    }
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(view.catalog.text("panel.ai"))
                .borders(Borders::ALL),
        ),
        area,
    );
}

fn render_logs(frame: &mut Frame<'_>, area: Rect, view: &AppView<'_>) {
    let width = area.width.saturating_sub(4) as usize;
    let items = view
        .logs
        .iter()
        .rev()
        .take(area.height.saturating_sub(2) as usize)
        .flat_map(|log| {
            wrap_to_width(&format!("[{}] {}", log.tick, log.body), width)
                .into_iter()
                .map(|line| ListItem::new(line).style(level_style(&log.level)))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .title(view.catalog.text("panel.logs"))
                .borders(Borders::ALL),
        ),
        area,
    );
}

fn render_activity(frame: &mut Frame<'_>, area: Rect, view: &AppView<'_>) {
    let width = area.width.saturating_sub(4) as usize;
    let items = view
        .ui_events
        .iter()
        .rev()
        .filter(|event| event.is_visible(view.current_frame))
        .take(area.height.saturating_sub(2) as usize)
        .map(|event| {
            let tag = ui_event_kind_label(view.catalog, &event.kind);
            let message = localized_ui_event_message(view.catalog, event);
            let marker = if event.intensity(view.current_frame) > 0.66 {
                "!"
            } else if event.intensity(view.current_frame) > 0.33 {
                ">"
            } else {
                "-"
            };
            let text = format!("[{}] {} {} {}", event.tick, marker, tag, message);
            ListItem::new(truncate_to_width(&text, width))
                .style(Theme::style(Theme::role_for_event(event)))
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .title(view.catalog.text("panel.activity"))
                .borders(Borders::ALL),
        ),
        area,
    );
}

fn render_decisions(frame: &mut Frame<'_>, area: Rect, view: &AppView<'_>) {
    let width = area.width.saturating_sub(4) as usize;
    let items = view
        .decisions
        .iter()
        .rev()
        .take(area.height.saturating_sub(2) as usize)
        .map(|decision| {
            let text = format!(
                "[{}] {} · {} · {}",
                decision.tick,
                action_label(view.catalog, &decision.action_id),
                decision_source_label(view.catalog, &decision.source),
                decision.reason
            );
            ListItem::new(truncate_to_width(&text, width)).style(
                if decision.source == DecisionSource::Fallback {
                    Theme::style(StyleRole::Warning)
                } else {
                    Theme::style(StyleRole::Text)
                },
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .title(view.catalog.text("panel.decisions"))
                .borders(Borders::ALL),
        ),
        area,
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, view: &AppView<'_>) {
    let text = format!(
        "{} · {} · {} {} · {} {}",
        view.catalog.text("control.quit"),
        view.catalog.text("control.pause"),
        view.catalog.text("label.run"),
        &view.run_id[..8],
        view.catalog.text("label.config"),
        view.config_hash
    );
    frame.render_widget(
        Paragraph::new(text)
            .style(Theme::style(StyleRole::Muted))
            .block(
                Block::default()
                    .title(view.catalog.text("panel.controls"))
                    .borders(Borders::ALL),
            ),
        area,
    );
}

fn resource_line(view: &AppView<'_>, key: &str, value: f64, unit: &str) -> Line<'static> {
    let label = pad_to_width(&view.catalog.text(key), 8, Align::Left);
    Line::from(format!("{label} {value:>5.2}{unit}"))
}

fn environment_line(view: &AppView<'_>, label_key: &str, value: String) -> Line<'static> {
    let label = pad_to_width(&view.catalog.text(label_key), 8, Align::Left);
    Line::from(format!("{label} {value}"))
}

fn level_style(level: &str) -> Style {
    match level {
        "danger" | "critical" => Theme::style(StyleRole::Critical),
        "warning" => Theme::style(StyleRole::Warning),
        "notice" => Theme::style(StyleRole::Recovery),
        _ => Theme::style(StyleRole::Text),
    }
}

fn ui_event_kind_label(catalog: &Catalog, kind: &UiEventKind) -> String {
    let key = match kind {
        UiEventKind::ValueDelta => "ui.kind.value_delta",
        UiEventKind::Risk => "ui.kind.risk",
        UiEventKind::Decision => "ui.kind.decision",
        UiEventKind::Fallback => "ui.kind.fallback",
        UiEventKind::Log => "ui.kind.log",
        UiEventKind::WorldEvent => "ui.kind.world_event",
        UiEventKind::Control => "ui.kind.control",
    };
    catalog.text(key)
}

fn localized_ui_event_message(catalog: &Catalog, event: &UiEvent) -> String {
    match &event.kind {
        UiEventKind::ValueDelta => {
            let delta = event.delta.unwrap_or_default();
            let sign = if delta >= 0.0 { "+" } else { "" };
            catalog.format(
                "ui.message.value_delta",
                &[
                    ("target", metric_label(catalog, &event.target)),
                    ("delta", format!("{sign}{delta:.2}")),
                ],
            )
        }
        UiEventKind::Risk => {
            catalog.format("ui.message.risk", &[("risk", risk_label(catalog, event))])
        }
        UiEventKind::Decision => catalog.format(
            "ui.message.decision",
            &[("action", action_label(catalog, &event.target))],
        ),
        UiEventKind::Fallback => catalog.format(
            "ui.message.fallback",
            &[("action", action_label(catalog, &event.target))],
        ),
        UiEventKind::Log | UiEventKind::WorldEvent | UiEventKind::Control => event.message.clone(),
    }
}

fn metric_label(catalog: &Catalog, target: &str) -> String {
    let key = match target {
        "hp" => "stat.hp",
        "hunger" => "stat.hunger",
        "thirst" => "stat.thirst",
        "energy" => "stat.energy",
        "morale" => "stat.morale",
        "raft" => "stat.raft",
        "food" => "resource.food",
        "water" => "resource.water",
        "wood" => "resource.wood",
        "fiber" => "resource.fiber",
        "tool" => "resource.tool",
        "distance_to_land" => "nav.target",
        _ => target,
    };
    catalog.text(key)
}

fn action_label(catalog: &Catalog, action_id: &str) -> String {
    let key = if action_id.starts_with("action.") {
        action_id.to_string()
    } else {
        format!("action.{action_id}")
    };
    let label = catalog.text(&key);
    if label == key {
        action_id.to_string()
    } else {
        label
    }
}

fn decision_source_label(catalog: &Catalog, source: &DecisionSource) -> String {
    match source {
        DecisionSource::Agent => catalog.text("source.agent"),
        DecisionSource::Fallback => catalog.text("source.fallback"),
    }
}

fn risk_label(catalog: &Catalog, event: &UiEvent) -> String {
    let key = if event.target.starts_with("risk.") {
        event.target.as_str()
    } else {
        legacy_risk_key(&event.message).unwrap_or("risk.medium")
    };
    catalog.text(key)
}

fn legacy_risk_key(message: &str) -> Option<&'static str> {
    let risk = message.split_whitespace().last()?;
    match risk {
        "low" => Some("risk.low"),
        "medium" => Some("risk.medium"),
        "high" => Some("risk.high"),
        "critical" => Some("risk.critical"),
        _ => None,
    }
}
