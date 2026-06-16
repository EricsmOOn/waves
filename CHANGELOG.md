# Changelog

All notable changes to Waves will be documented in this file.

## [0.1.0] — 2026-06-16

### Added

- Core runtime engine with tick-based game loop, event triggering, and action resolution.
- `Scenario` trait with `build_scenario()` dispatching.
- `sea_survival` scenario — ocean survival with stats, resources, 9 actions, 8 events, 66 balance parameters.
- MCP stdio server exposing 7 tools (`waves_start_run`, `waves_get_state`, `waves_step`, `waves_get_pending_decision`, `waves_submit_decision`, `waves_pause`, `waves_resume`).
- Shared local daemon with Unix-domain-socket RPC for multi-terminal setup (server + MCP + TUI).
- Ratatui-based TUI observer with status gauges, environment panel, AI panel, activity feed, logs, and decision list.
- SQLite persistence with WAL mode — runs, snapshots, domain events, decisions, logs, and UI events.
- Replay summary from SQLite.
- CSV-driven localization catalog with `zh-CN` (default) and `en-US` locales.
- Config validation (`validate` subcommand) checking duplicate IDs, bounds, resolver references, and locale keys.
- CLI with 7 subcommands: `validate`, `inspect`, `run`, `tui`, `serve`, `replay`, `mcp`.
- Unicode-aware text width helpers for CJK TUI rendering.
- UI event system with priority levels, motion types, and timed visibility.
- Threat-based theme mapping for TUI color roles.
- Integration test suite (37 tests across 12 test suites).
- Design documentation (14 docs covering architecture, UX spec, AI contract, persistence, and agent playbook).
- Bilingual README (Chinese and English).
