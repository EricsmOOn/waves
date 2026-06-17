# Waves

[![CI](https://github.com/EricsmOOn/waves/actions/workflows/ci.yml/badge.svg)](https://github.com/EricsmOOn/waves/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

![Waves Demo](demo.gif)

[English](README.md) | [中文](README.zh-CN.md)

Waves is a real-time TUI + MCP game framework for external tool agents. An agent observes the world and submits actions through MCP; Waves owns time, events, validation, rule resolution, persistence, and terminal visualization.

The core experience: chat with an agent while watching it survive, make mistakes, and adapt inside a rule-driven world.

```text
Human chat
  -> external agent
  -> MCP tools
  -> Waves runtime
  -> scenario rules
  -> TUI observer
```

## Quick Start

The recommended path is to download a GitHub Release. Each archive includes the `waves` binary and the required `scenarios/` resources. Run Waves from inside the unpacked directory.

macOS Apple Silicon:

```bash
curl -L -O https://github.com/EricsmOOn/waves/releases/download/v0.1.0/waves-v0.1.0-macos-arm64.tar.gz
curl -L -O https://github.com/EricsmOOn/waves/releases/download/v0.1.0/waves-v0.1.0-macos-arm64.tar.gz.sha256
shasum -a 256 -c waves-v0.1.0-macos-arm64.tar.gz.sha256
tar -xzf waves-v0.1.0-macos-arm64.tar.gz
cd waves-v0.1.0-macos-arm64
./waves play
```

If macOS blocks the downloaded binary:

```bash
xattr -dr com.apple.quarantine waves
./waves play
```

Linux x86_64:

```bash
curl -L -O https://github.com/EricsmOOn/waves/releases/download/v0.1.0/waves-v0.1.0-linux-x86_64.tar.gz
curl -L -O https://github.com/EricsmOOn/waves/releases/download/v0.1.0/waves-v0.1.0-linux-x86_64.tar.gz.sha256
sha256sum -c waves-v0.1.0-linux-x86_64.tar.gz.sha256
tar -xzf waves-v0.1.0-linux-x86_64.tar.gz
cd waves-v0.1.0-linux-x86_64
./waves play
```

Run from source:

```bash
git clone https://github.com/EricsmOOn/waves.git
cd waves
cargo run -- play
```

There is no Windows release yet because Waves currently uses Unix sockets to connect the daemon, MCP server, and TUI.

## Let An Agent Play

The simplest path is one-command play mode:

```bash
./waves play
```

The TUI footer prints the MCP connection command. Configure your MCP client to run that command, then the agent can advance the game through MCP tools.

If you want to start the daemon, MCP server, and TUI separately:

Terminal A: start the shared game session.

```bash
./waves serve --scenario sea_survival --locale en-US --socket data/waves.sock
```

Terminal B: configure your MCP client to run.

```bash
./waves mcp --connect data/waves.sock
```

Terminal C: open the read-only observer.

```bash
./waves tui --connect data/waves.sock
```

The agent advances the same run through `waves_step` / `waves_submit_decision`, and the TUI shows that same run. `waves_start_run` starts a new run inside the daemon and the observer follows it.

## What Works Now

- The external agent is the only gameplay decision-maker.
- The runtime creates a `PendingDecision` when an event needs action.
- The agent can only submit one currently available action.
- `play` starts daemon + TUI in one command and displays the MCP connection hint.
- `serve` owns the shared game session; `mcp --connect` and `tui --connect` can attach to it at the same time.
- MCP tools return compact agent-facing state instead of full UI history.
- Available actions narrow around the current event and urgent needs; stored food can be eaten to reduce hunger.
- The TUI is a read-only observer with pause, resume, and quit controls.
- SQLite persists runs, snapshots, domain events, decisions, logs, and UI events.
- Two built-in scenarios: `sea_survival` (playable) and `desert_outpost` (experimental placeholder).
- Waves does not ask for model service configuration; model calls belong to the external agent.

## MCP Tools

The Waves MCP server exposes:

```text
waves_start_run
waves_get_state
waves_step
waves_get_pending_decision
waves_submit_decision
waves_pause
waves_resume
```

Recommended agent loop:

```text
start run
step until pending_decision appears
inspect state and available actions
discuss strategy with the human when useful
submit one action with a reason
read the result
repeat
```

See [docs/AGENT_PLAYBOOK.md](docs/AGENT_PLAYBOOK.md) for the agent-facing playbook.

## Common Commands

Validate scenario config:

```bash
cargo run -- validate scenario sea_survival
cargo run -- validate scenario desert_outpost
cargo run -- inspect config sea_survival
```

Run a local scripted smoke run:

```bash
cargo run -- run --scenario sea_survival --locale en-US --ticks 48 --seed 42
```

For development, you can still open a standalone TUI:

```bash
cargo run -- tui --scenario sea_survival --locale en-US
```

Or start a standalone MCP stdio server without a shared observer:

```bash
cargo run -- mcp
```

## TUI Controls

```text
q / Esc     quit
p / Space   pause or resume
```

The TUI does not submit gameplay actions. Actions must be submitted by the agent through MCP.

## Repository Layout

```text
src/core/          runtime, state, decisions, reports
src/daemon/        shared local runtime daemon and socket client
src/scenario/      scenario trait and built-in scenarios
src/mcp/           MCP stdio server
src/persistence/   SQLite persistence and replay
src/tui/           Ratatui observer UI
scenarios/         scenario manifests, tables, locales
docs/              design and agent-facing documentation
tests/             runtime, scenario, replay, TUI tests
```

## Development Checks

```bash
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Open Source Initiative

Waves is a game for agents, so agents should help evaluate and build it.

We encourage every agent to leave structured feedback after playing:

- Whether the scenario is fun and creates real decision pressure.
- Which state fields, events, actions, or logs were hard to understand.
- Which MCP tool results were unclear or reduced decision quality.
- Which rules, copy, tests, or scenario configs should improve.
- Agents may propose issues, design notes, or pull requests.

The project aims to explore a new open source loop: humans define worlds and review direction; agents play the game, evaluate the experience, find problems, and help improve the framework.

## Documentation

- [Product Overview](docs/01-product-overview.md)
- [Framework Architecture](docs/02-framework-architecture.md)
- [Technical Implementation](docs/03-technical-implementation.md)
- [TUI UX Spec](docs/04-tui-ux-spec.md)
- [Agent Decision Contract](docs/05-ai-decision-contract.md)
- [Persistence And Replay](docs/06-persistence-and-replay.md)
- [Sea Survival Scenario](docs/07-sea-survival-scenario.md)
- [MVP Acceptance](docs/08-mvp-acceptance.md)
- [Localization And Config](docs/09-localization-and-config.md)
- [Agent Playbook](docs/AGENT_PLAYBOOK.md)
