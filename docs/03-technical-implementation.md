# Technical Implementation

## 最终技术方案

```text
Language: Rust
TUI: Ratatui + crossterm
Agent Control: MCP stdio
Persistence: SQLite WAL
Data Model: event sourcing + periodic snapshots
Serialization: serde + serde_json
Config: TOML manifest + CSV/TSV data tables
Localization: locale tables + display-width aware TUI layout
Architecture: local daemon + stdio MCP bridge + TUI observer
Scenario: sea_survival as first application
Testing: cargo test + deterministic replay tests
```

## 为什么使用 Rust

本项目的核心是一个长期运行的 terminal-native runtime，而不是一次性 CLI。

Rust 适合本项目的原因：

```text
常驻进程稳定
tick 调度可靠
类型系统适合定义 Scenario / Event / Action / Resolver
单文件分发方便
Ratatui 对复杂终端界面控制力强
性能余量足够支持动效、回放和长日志
```

主要代价：

```text
开发速度慢于 TypeScript
TUI 组件需要自己组织更多
scenario 插件化需要谨慎设计
```

结论：框架内核用 Rust。示例应用先使用 Rust rule module + TOML manifest + CSV/TSV 配置表。等出现第二个或第三个 scenario 后，再评估 Lua、Rhai 或 WASM 插件。

## TUI 框架

采用：

```text
Ratatui + crossterm
```

Ratatui 负责布局和绘制，crossterm 负责终端事件、键盘输入和 raw mode。这个组合适合复杂面板、局部状态渲染、低资源常驻运行和精细动效控制。

当前实现包含一个 `UiEvent` 流。runtime 在世界事件、agent 决策、规则结算和日志生成后派生 `UiEvent`，TUI 的 Activity 区域消费这些事件。`UiEvent` 只服务表现层和回放调试，不反向修改世界状态。

## 是否前后端分离

第一版不做网络意义上的前后端分离，但会把可观察游戏会话放进本地 daemon。

采用：

```text
local runtime daemon
├─ owns one RuntimeSession
├─ exposes a Unix socket JSON-lines RPC
├─ accepts MCP bridge requests
└─ accepts TUI observer polling
```

原因：

```text
MCP stdio 足够支撑 Codex/Claude 类 agent 本地控制
agent 玩、用户旁观需要 MCP 和 TUI 共享同一局游戏
本地 Unix socket 比引入 HTTP server/WebSocket 更小
MVP 重点是验证 runtime、scenario、agent 决策和 TUI 体验
事件总线和持久化边界可以为未来分离预留接口
```

仍保留单进程 `tui` 和不带 `--connect` 的 `mcp`，用于开发、smoke test 和没有旁观窗口的 agent session。

未来扩展方向：

```text
runtime daemon
web dashboard
remote observer
WebSocket event stream
multi-run comparison
```

## 建议工程结构

```text
src/
  main.rs
  app.rs
  core/
    mod.rs
    runtime.rs
    clock.rs
    event_bus.rs
    state.rs
    action.rs
    resolution.rs
    memory.rs
    config_schema.rs
  scenario/
    mod.rs
    sea_survival/
      mod.rs
    desert_outpost/
      mod.rs
  mcp/
    mod.rs
  daemon/
    mod.rs
  persistence/
    mod.rs
    sqlite.rs
    snapshot.rs
    replay.rs
  tui/
    mod.rs
    renderer.rs
    layout.rs
    theme.rs
    animation.rs
    ui_events.rs
    text_width.rs
  i18n/
    mod.rs
    catalog.rs
    formatter.rs
  config/
    mod.rs
    loader.rs
    validator.rs
    tables.rs
tests/
  runtime_tests.rs
  replay_tests.rs
  sea_survival_tests.rs
  config_validation_tests.rs
  tui_text_width_tests.rs
scenarios/
  sea_survival/
    scenario.toml
    tables/
      stats.csv
      resources.csv
      actions.csv
      events.csv
      event_weights.csv
      balance.csv
      panels.csv
    locales/
      zh-CN.csv
      en-US.csv
  desert_outpost/
    scenario.toml
    tables/
    locales/
```

## 关键依赖建议

```toml
ratatui = "TUI rendering"
crossterm = "terminal input/output"
serde = "serialization"
serde_json = "JSON payloads and snapshots"
toml = "scenario manifest"
csv = "designer-editable data tables"
rusqlite = "SQLite access"
anyhow = "application errors"
thiserror = "domain errors"
tracing = "structured logs"
rand = "deterministic resolution RNG"
uuid = "run ids"
chrono or time = "timestamps"
unicode-width = "terminal display width"
unicode-segmentation = "grapheme-safe text handling"
```

SQLite 访问可在 `sqlx` 和 `rusqlite` 中二选一。若想简单稳定，优先 `rusqlite`；若要 async 和编译期 SQL 检查，选择 `sqlx`。

## 当前 CLI

```bash
cargo run -- validate scenario sea_survival
cargo run -- validate scenario desert_outpost
cargo run -- inspect config sea_survival
cargo run -- inspect config desert_outpost
cargo run -- mcp
cargo run -- mcp --connect data/waves.sock
cargo run -- serve --scenario sea_survival --locale zh-CN --socket data/waves.sock
cargo run -- run --scenario sea_survival --locale zh-CN --ticks 48
cargo run -- run --scenario desert_outpost --locale zh-CN --ticks 8
cargo run -- replay --run-id <run_id> --db data/waves.sqlite
cargo run -- tui --scenario sea_survival --locale zh-CN
cargo run -- tui --connect data/waves.sock
```

`validate` 输出配置错误时使用 `file:row:column message`。`inspect config` 输出 scenario 版本、config hash、表行数、locale key 数、balance key 数和 resolver 注册数量。`serve` 启动共享 runtime daemon。`mcp --connect` 将 MCP tool call 转发到 daemon，并把 daemon 的完整快照压缩成 agent-facing compact 响应；不带 `--connect` 时启动独立 stdio MCP session。`tui --connect` 只读观察 daemon 中的同一个 run，并继续使用完整 UI 事件流；不带 `--connect` 时启动单机 TUI。`run` 使用本地 scripted runner 做 smoke test 和生成 replay 数据，不是内置模型模式。`replay` 当前输出保存 run 的摘要，不调用模型。

## 测试策略

```text
core runtime：tick、事件调度、pending decision、外部提交、暂停恢复
scenario：行动前置条件、结算公式、胜负条件
persistence：事件追加、快照恢复、WAL 写入
replay：同 seed 同事件流得到同结果
TUI：UiEvent 生成规则、数值 delta 映射、危险等级映射
config：必填字段、引用完整性、数值范围、重复 id
i18n：缺失 key、变量占位符、fallback locale
中文排版：中英文混排宽度、截断、换行、对齐
```

确定性测试要求：

```text
固定 RNG seed
固定 tick delta
使用 scripted runner
回放结果可重复
```
