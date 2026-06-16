# Contributing to Waves

Waves is a game framework built for agents, and we welcome contributions from both humans and agents.

## Development Setup

You need Rust stable (1.85+). Install via [rustup](https://rustup.rs).

```bash
git clone https://github.com/EricsmOOn/waves.git
cd waves
cargo build
cargo test
```

### Pre-flight Checks

Before submitting a PR:

```bash
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

All three must pass.

## Project Layout

```
src/core/          runtime engine, game state, decision loop
src/daemon/        shared local daemon and socket client
src/scenario/      Scenario trait and built-in scenarios
src/mcp/           MCP stdio server
src/persistence/   SQLite storage and replay
src/tui/           Ratatui observer UI
src/i18n/          localization catalog
scenarios/         scenario manifests, CSV tables, locale data
docs/              design docs and agent-facing playbook
tests/             integration tests
```

## Adding a New Scenario

1. Create `scenarios/<name>/scenario.toml` — manifest with id, version, entry, default_locale.
2. Add CSV tables: `stats.csv`, `resources.csv`, `actions.csv`, `events.csv`, `balance.csv`, `panels.csv`, `prompts.csv`.
3. Add locale CSVs under `scenarios/<name>/locales/`.
4. Implement the `Scenario` trait in `src/scenario/<name>/mod.rs`.
5. Register it in `src/scenario/mod.rs` → `build_scenario()`.
6. Add integration tests under `tests/`.
7. Validate with `cargo run -- validate scenario <name>`.

## Code Style

- Follow `cargo fmt` and `cargo clippy` defaults.
- Prefer `expect("why this is ok")` over bare `unwrap()`.
- Keep `unsafe` to an absolute minimum (currently zero).
- Write tests alongside new functionality.

## Agent Contributions

Agents playing Waves are encouraged to contribute:

- Scenario balance feedback and suggestions.
- Bug reports and reproduction steps.
- Documentation improvements.
- Locale corrections and additions.
- Test cases for edge behaviors observed during play.

Agent-generated PRs should follow the same review standards as human contributions. Use the design docs in `docs/` as reference.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
