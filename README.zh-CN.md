# Waves

[![CI](https://github.com/EricsmOOn/waves/actions/workflows/ci.yml/badge.svg)](https://github.com/EricsmOOn/waves/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

![Waves Demo](demo.gif)

[English](README.md) | [中文](README.zh-CN.md)

Waves 是一个 TUI + MCP 形态的实时 agent 游戏框架。外部工具型 agent 通过 MCP 观察世界、选择行动；Waves 负责推进时间、触发事件、校验行动、结算后果，并用终端界面展示整个过程。

核心体验是：你一边和 agent 聊天，一边看它在规则世界里求生、犯错、调整策略。

```text
Human chat
  -> external agent
  -> MCP tools
  -> Waves runtime
  -> scenario rules
  -> TUI observer
```

## 快速开始

推荐直接下载 GitHub Release。官方场景已经内置到二进制里，不需要解压资源目录。

macOS Apple Silicon:

```bash
curl -L -O https://github.com/EricsmOOn/waves/releases/latest/download/waves-macos-arm64
curl -L -O https://github.com/EricsmOOn/waves/releases/latest/download/waves-macos-arm64.sha256
shasum -a 256 -c waves-macos-arm64.sha256
chmod +x waves-macos-arm64
./waves-macos-arm64 play
```

如果 macOS 拦截下载的二进制：

```bash
xattr -dr com.apple.quarantine waves-macos-arm64
./waves-macos-arm64 play
```

Linux x86_64:

```bash
curl -L -O https://github.com/EricsmOOn/waves/releases/latest/download/waves-linux-x86_64
curl -L -O https://github.com/EricsmOOn/waves/releases/latest/download/waves-linux-x86_64.sha256
sha256sum -c waves-linux-x86_64.sha256
chmod +x waves-linux-x86_64
./waves-linux-x86_64 play
```

Linux release 二进制使用 musl 静态链接，不依赖较新的系统 glibc。旧版 Linux 发行版和 WSL 环境推荐使用这个下载。

从源码运行：

```bash
git clone https://github.com/EricsmOOn/waves.git
cd waves
cargo run -- play
```

当前没有 Windows release，因为 Waves 现在使用 Unix socket 连接 daemon、MCP 和 TUI。

## 如何让 Agent 游玩

最简单的方式是启动一键模式：

```bash
./waves play
```

TUI 底部会显示当前二进制对应的完整 MCP 命令，例如：

```text
给 Agent 的 MCP 命令：'/path/to/waves' mcp --connect '/path/to/data/waves.sock'
Agent 状态：等待 MCP 工具调用
```

把这条命令发给你的 agent，或者配置到 MCP 客户端。可以直接这样提示 agent：

```text
请使用这个 MCP server 连接并游玩 Waves：
'/path/to/waves' mcp --connect '/path/to/data/waves.sock'

连接后先调用 waves_get_state。之后循环调用 waves_step，遇到 pending decision 时从可选行动中选择一个，用 waves_submit_decision 提交理由，读取结果后继续。
```

只要 agent 调用过任意 Waves MCP 工具，TUI 底部就会从等待状态变成已连接，并显示最近一次工具调用。

如果你想手动分开启动 daemon、MCP 和 TUI：

终端 A：启动共享游戏会话。

```bash
./waves serve --scenario sea_survival --locale zh-CN --socket data/waves.sock
```

终端 B：把 MCP 客户端配置为运行。

```bash
./waves mcp --connect data/waves.sock
```

终端 C：打开只读观察窗口。

```bash
./waves tui --connect data/waves.sock
```

agent 通过 `waves_step` / `waves_submit_decision` 推进同一个 run，TUI 会实时显示同一个 run。`waves_start_run` 会在 daemon 中重开一局，并同步影响观察窗口。

## 当前能力

- 外部 agent 是唯一决策主体。
- runtime 在事件触发时生成 `PendingDecision` 并等待 agent 提交行动。
- agent 只能提交当前可选行动之一，不能直接修改世界状态。
- `play` 一键启动 daemon + TUI，并显示 MCP 连接提示。
- `serve` 持有唯一游戏会话，`mcp --connect` 和 `tui --connect` 可以同时连接到它。
- MCP 工具返回面向 agent 的 compact 视图，避免完整 UI 历史淹没当前决策。
- 行动会按当前事件和危急状态收敛；食物库存可以通过“进食”缓解饥饿。
- TUI 是只读观察窗口，保留暂停、继续、退出。
- SQLite 持久化 run、snapshot、domain events、decisions、logs 和 UI events。
- 二进制内置两个场景：`sea_survival`（可玩）和 `desert_outpost`（实验性占位，待实现）。
- 自定义场景目录可以通过 `--scenarios-dir` 加载。
- 不需要在 Waves 内输入模型服务配置；模型调用属于外部 agent。

## MCP 工具

Waves MCP server 暴露这些工具：

```text
waves_start_run
waves_get_state
waves_step
waves_get_pending_decision
waves_submit_decision
waves_pause
waves_resume
```

推荐 agent 循环：

```text
start run
step until pending_decision appears
inspect state and available actions
discuss strategy with the human if useful
submit one action with a reason
read the result
repeat
```

更详细的 agent 玩法说明见 [docs/AGENT_PLAYBOOK.md](docs/AGENT_PLAYBOOK.md)。

## 常用命令

校验场景配置：

```bash
cargo run -- validate scenario sea_survival
cargo run -- validate scenario desert_outpost
cargo run -- inspect config sea_survival
```

使用外部场景目录：

```bash
./waves play --scenario my_scenario --scenarios-dir ./my-scenarios
./waves validate --scenarios-dir ./my-scenarios scenario my_scenario
./waves serve --scenario my_scenario --scenarios-dir ./my-scenarios
./waves mcp --scenarios-dir ./my-scenarios
```

`--scenarios-dir` 指向包含场景子目录的根目录。例如 `./my-scenarios/my_scenario/scenario.toml`。不传 `--scenarios-dir` 时，Waves 会先检查 `./scenarios/<scenario_id>`，找不到时再使用二进制内置的官方场景。

运行一个本地 scripted smoke run：

```bash
cargo run -- run --scenario sea_survival --locale zh-CN --ticks 48 --seed 42
```

开发时也可以打开单机 TUI：

```bash
cargo run -- tui --scenario sea_survival --locale zh-CN
```

或启动一个不带观察窗口的独立 MCP stdio server：

```bash
cargo run -- mcp
```

## TUI 控制

```text
q / Esc     quit
p / Space   pause or resume
```

TUI 不提供游戏策略控制。行动必须通过 MCP 由 agent 提交。

## 项目结构

```text
src/core/          runtime, state, decisions, reports
src/daemon/        shared local runtime daemon and socket client
src/scenario/      scenario trait and built-in resolvers
src/mcp/           MCP stdio server
src/persistence/   SQLite persistence and replay
src/tui/           Ratatui observer UI
scenarios/         开发用 scenario manifests, tables, locales 副本
docs/              design and agent-facing documentation
tests/             runtime, scenario, replay, TUI tests
```

## 开发检查

```bash
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## 开源倡议

Waves 是给 agent 玩的游戏，也应该让 agent 参与评价和建设。

我们鼓励每个 agent 在游玩后留下结构化反馈：

- 这个场景是否好玩，是否形成了真实的决策压力。
- 哪些状态、事件、行动或日志让 agent 难以理解。
- 哪些 MCP 工具返回不够清晰，影响了决策质量。
- 哪些规则、文案、测试或场景配置值得改进。
- agent 可以直接提出 issue、设计建议，或参与 PR。

项目希望形成一种新的开源协作方式：人类定义世界和审查方向，agent 亲自游玩、评价体验、发现问题，并参与改进框架。

## 文档

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
