use crate::config::load_scenario_config;
use crate::core::RuntimeSession;
use crate::daemon::{DaemonClient, SessionSnapshot};
use crate::i18n::Catalog;
use crate::tui::{AppView, render_app};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use serde_json::json;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

type AppTerminal = Terminal<CrosstermBackend<io::Stdout>>;

pub fn run_tui_remote_with_hint(
    socket_path: PathBuf,
    tick_rate: Duration,
    connection_hint: String,
) -> Result<()> {
    let client = DaemonClient::new(socket_path);
    let mut catalog_cache = RemoteCatalogCache::default();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_tui_remote_loop_with_hint(
        &mut terminal,
        &client,
        &mut catalog_cache,
        tick_rate,
        &connection_hint,
    );
    let cleanup = restore_terminal(&mut terminal);
    result.and(cleanup)
}

pub fn run_tui(mut session: RuntimeSession, tick_rate: Duration) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_tui_loop(&mut terminal, &mut session, tick_rate);
    let cleanup = restore_terminal(&mut terminal);
    result.and(cleanup)
}

fn run_tui_loop(
    terminal: &mut AppTerminal,
    session: &mut RuntimeSession,
    tick_rate: Duration,
) -> Result<()> {
    let mut last_tick = Instant::now();

    loop {
        session.advance_presentation_frame();
        terminal.draw(|frame| {
            let view = AppView {
                run_id: &session.run_id,
                scenario_id: &session.config.manifest.id,
                config_hash: &session.config.config_hash,
                state: &session.state,
                catalog: &session.catalog,
                logs: &session.logs,
                decisions: &session.decisions,
                ui_events: &session.ui_events,
                pending_decision: session.pending_decision(),
                current_frame: session.presentation_frame,
                paused: session.paused,
                connection_hint: None,
            };
            render_app(frame, &view);
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0));
        if event::poll(timeout.min(Duration::from_millis(100)))?
            && let Event::Key(key) = event::read()?
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                KeyCode::Char('p') | KeyCode::Char(' ') => {
                    session.toggle_pause()?;
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            if !session.paused {
                session.step()?;
            }
            last_tick = Instant::now();
        }
    }
}

pub fn run_tui_remote(socket_path: PathBuf, tick_rate: Duration) -> Result<()> {
    let client = DaemonClient::new(socket_path);
    let mut catalog_cache = RemoteCatalogCache::default();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_tui_remote_loop(&mut terminal, &client, &mut catalog_cache, tick_rate);
    let cleanup = restore_terminal(&mut terminal);
    result.and(cleanup)
}

fn run_tui_remote_loop_with_hint(
    terminal: &mut AppTerminal,
    client: &DaemonClient,
    catalog_cache: &mut RemoteCatalogCache,
    tick_rate: Duration,
    connection_hint: &str,
) -> Result<()> {
    loop {
        let snapshot: SessionSnapshot =
            client.request("get_state", json!({ "advance_frame": true }))?;
        let catalog = catalog_cache.catalog_for(&snapshot)?;

        terminal.draw(|frame| {
            let view = AppView {
                run_id: &snapshot.run_id,
                scenario_id: &snapshot.scenario_id,
                config_hash: &snapshot.config_hash,
                state: &snapshot.state,
                catalog,
                logs: &snapshot.logs,
                decisions: &snapshot.decisions,
                ui_events: &snapshot.ui_events,
                pending_decision: snapshot.pending_decision.as_ref(),
                current_frame: snapshot.presentation_frame,
                paused: snapshot.paused,
                connection_hint: Some(connection_hint),
            };
            render_app(frame, &view);
        })?;

        if event::poll(tick_rate.min(Duration::from_millis(250)))?
            && let Event::Key(key) = event::read()?
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                KeyCode::Char('p') | KeyCode::Char(' ') => {
                    if snapshot.paused {
                        client.request_value("resume", json!({}))?;
                    } else {
                        client.request_value("pause", json!({}))?;
                    }
                }
                _ => {}
            }
        }
    }
}

fn run_tui_remote_loop(
    terminal: &mut AppTerminal,
    client: &DaemonClient,
    catalog_cache: &mut RemoteCatalogCache,
    tick_rate: Duration,
) -> Result<()> {
    loop {
        let snapshot: SessionSnapshot =
            client.request("get_state", json!({ "advance_frame": true }))?;
        let catalog = catalog_cache.catalog_for(&snapshot)?;

        terminal.draw(|frame| {
            let view = AppView {
                run_id: &snapshot.run_id,
                scenario_id: &snapshot.scenario_id,
                config_hash: &snapshot.config_hash,
                state: &snapshot.state,
                catalog,
                logs: &snapshot.logs,
                decisions: &snapshot.decisions,
                ui_events: &snapshot.ui_events,
                pending_decision: snapshot.pending_decision.as_ref(),
                current_frame: snapshot.presentation_frame,
                paused: snapshot.paused,
                connection_hint: None,
            };
            render_app(frame, &view);
        })?;

        if event::poll(tick_rate.min(Duration::from_millis(250)))?
            && let Event::Key(key) = event::read()?
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                KeyCode::Char('p') | KeyCode::Char(' ') => {
                    if snapshot.paused {
                        client.request_value("resume", json!({}))?;
                    } else {
                        client.request_value("pause", json!({}))?;
                    }
                }
                _ => {}
            }
        }
    }
}

fn restore_terminal(terminal: &mut AppTerminal) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

#[derive(Default)]
struct RemoteCatalogCache {
    key: Option<(String, String, String)>,
    catalog: Option<Catalog>,
}

impl RemoteCatalogCache {
    fn catalog_for(&mut self, snapshot: &SessionSnapshot) -> Result<&Catalog> {
        let key = (
            snapshot.scenario_id.clone(),
            snapshot.active_locale.clone(),
            snapshot.config_hash.clone(),
        );
        if self.key.as_ref() != Some(&key) {
            let config = load_scenario_config(&snapshot.scenario_id)?;
            self.catalog = Some(Catalog::new(
                snapshot.active_locale.clone(),
                config.manifest.default_locale,
                config.locales,
            ));
            self.key = Some(key);
        }
        Ok(self.catalog.as_ref().expect("catalog initialized above"))
    }
}
