# Waves — 让 AI 在终端里玩生存游戏

[![CI](https://github.com/EricsmOOn/waves/actions/workflows/ci.yml/badge.svg)](https://github.com/EricsmOOn/waves/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

![Waves Demo](demo.gif)

> Waves 是一个**终端游戏框架**，但它的玩家不是人类，而是 **AI agent**。
> 你打开 TUI 观察窗口，看着 agent 在规则世界里挣扎求生、做决策、犯错、调整策略。
> 你甚至可以一边跟 agent 聊天，一边看它玩。

[中文](#中文) | [English](#english)

---

## 中文

### 这是什么

Waves 运行一个规则驱动的生存游戏。游戏的玩家是一个外部 AI agent（比如 Claude），它通过 [MCP](https://modelcontextprotocol.io) 协议连接进来，观察世界状态，选择行动，然后 Waves 结算后果。

**人类不操作游戏** —— 你是一个观众。你可以打开 TUI 窗口看 agent 玩，也可以跟 agent 聊天讨论策略。

```text
你 (人类)  ←→  AI agent  ←→  MCP  ←→  Waves 游戏世界
                     ↑
              你通过 TUI 观看
```

### 30 秒快速体验

需要 Rust stable（[安装 rustup](https://rustup.rs)）。

```bash
# 1. 编译
git clone https://github.com/EricsmOOn/waves.git
cd waves
cargo build

# 2. 打开 TUI 观察窗口（内置 fallback AI 会自动玩）
cargo run -- tui --scenario sea_survival --locale zh-CN
```

你会看到一个终端界面，左边是状态栏（血量、饥饿、口渴、体力），中间是事件日志和 AI 面板，右边是可选行动。**按 `q` 退出。**

> 内置的 fallback AI 会随机选行动，所以 agent 很快就会死掉。想让 agent 聪明地玩？继续往下看。

### 让 Claude 来玩

Waves 的最佳体验是让 Claude（或其他支持 MCP 的 agent）来玩游戏。你需要开三个终端：

**终端 A** —— 启动游戏服务器（持有唯一的游戏会话）：

```bash
cargo run -- serve --scenario sea_survival --locale zh-CN --socket data/waves.sock
```

**终端 B** —— 配置 Claude Code 的 MCP 连接。在 Claude Code 中运行：

```bash
# 在 Claude Code 里添加 MCP server
/claude mcp add waves -- cargo run -- mcp --connect data/waves.sock

# 或者手动编辑 ~/.claude/settings.json，添加：
# "mcpServers": {
#   "waves": {
#     "command": "cargo",
#     "args": ["run", "--", "mcp", "--connect", "data/waves.sock"]
#   }
# }
```

然后告诉 Claude：

```
现在开始玩 Waves 的 sea_survival 场景。用 waves_start_run 开始游戏，
然后用 waves_step 推进，当 pending_decision 出现时，分析状态选择最佳行动。
告诉我你的每一步推理和决策。
```

**终端 C** —— 打开 TUI 看 agent 玩：

```bash
cargo run -- tui --connect data/waves.sock
```

三个终端的关系：

```
终端 A: serve     ← 持有游戏实例
终端 B: mcp       ← agent 通过 MCP 工具操作游戏
终端 C: tui       ← 你观看游戏的窗口
```

> 📖 更详细的 agent 玩法说明见 [Agent Playbook](docs/AGENT_PLAYBOOK.md)。

### 游戏界面

```
┌─────────────────────────────────────────────────────┐
│ Waves 观测框架 · 海上求生                Day 3  ☀️     │
├──────────────┬──────────────────┬───────────────────┤
│ 状态         │ AI 面板           │ 可选行动           │
│ HP    ████░  │ 目标: 活下去       │ 🎣 钓鱼           │
│ 饥饿  ██░░░  │ 担忧: 缺水         │ 🍽️ 进食           │
│ 口渴  ████░  │                   │ 💧 收集雨水        │
│ 体力  ███░░  │ 事件日志           │ 🪵 打捞漂浮物       │
│              │ [Day 1] 暴风雨来袭  │ 🔧 修补木筏        │
│ 资源         │ [Day 2] 发现鱼群    │ 😴 休息           │
│ 食物  1.8天  │ [Day 3] 发现浮箱    │ 🌤️ 观察天气       │
│ 水    0.5天  │                   │ 🗺️ 研究海图        │
│ 木材  8      │ 操作日志           │ 🧭 改变航向        │
│              │ → 选择钓鱼, 获得2份  │                   │
│ 环境         │ → 进食消耗1份食物   │                   │
│ 晴天 · 平静   │ → 发现浮箱，选择    │                   │
│ 距离 120km   │   打捞，获得3木材   │                   │
├──────────────┴──────────────────┴───────────────────┤
│ q/Esc 退出  |  p/Space 暂停                          │
└─────────────────────────────────────────────────────┘
```

### 场景

| 场景 | 简介 | 状态 |
|------|------|------|
| `sea_survival` | 海上求生 —— 管理血量、饥饿、口渴、体力，应对天气和随机事件，撑到靠岸 | ✅ 可玩 |
| `desert_outpost` | 沙漠哨站（规划中） | 🚧 占位 |

每个场景的数据（属性、资源、行动、事件、平衡参数、文案）都是 CSV 驱动的，修改不需要改代码。见 `scenarios/` 目录。

### 游戏机制

- **时间推进**: 每个 tick 代表一段游戏时间，属性自动衰减
- **随机事件**: 天气变化、鱼群、风暴、浮箱……事件触发时 agent 必须做出选择
- **决策时刻**: 当 `pending_decision` 出现时，游戏暂停等待 agent 提交行动
- **行动收敛**: 可选行动会根据当前事件和紧急状态变化（饿了必须先吃东西）
- **胜负条件**: HP 归零 / 木筏损毁 = 失败，抵达陆地 = 胜利

### MCP 工具

Waves 向 agent 暴露 7 个 MCP 工具：

```text
waves_start_run     开始新一局
waves_get_state     获取当前状态（compact 视图，不含完整历史）
waves_step          推进一 tick
waves_get_pending_decision  查看当前等待的决策
waves_submit_decision       提交行动决策
waves_pause         暂停
waves_resume        继续
```

Agent 循环：`start → step → [pending_decision? → submit → step → …]`

### 开发

```bash
# 开发检查
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test

# 校验场景配置
cargo run -- validate scenario sea_survival

# 查看场景配置
cargo run -- inspect config sea_survival
```

```text
src/core/          游戏引擎 —— 时间推进、事件、决策结算
src/scenario/      场景接口和内置场景实现
src/daemon/        本地共享 daemon 和 socket 通信
src/mcp/           MCP stdio server
src/tui/           Ratatui 终端界面
src/persistence/   SQLite 持久化和 replay
src/i18n/          本地化目录
scenarios/         场景的 CSV 数据和文案
tests/             集成测试
```

更多细节见 [docs/](docs/)。

---

## English

### What Is This

Waves runs a rule-driven survival game inside your terminal. The player is an external AI agent (such as Claude), which connects via [MCP](https://modelcontextprotocol.io) to observe the world, choose actions, and see the consequences. Waves owns the clock, the rules, and the resolution.

**You don't play.** You watch. Open the TUI observer and see the agent survive, mess up, and adapt. You can also chat with the agent and discuss strategy while it plays.

```text
You (human)  ←→  AI agent  ←→  MCP  ←→  Waves game world
                     ↑
              You watch via TUI
```

### 30-Second Quick Start

Requires Rust stable ([install rustup](https://rustup.rs)).

```bash
# 1. Build
git clone https://github.com/EricsmOOn/waves.git
cd waves
cargo build

# 2. Open the TUI viewer (built-in fallback AI plays automatically)
cargo run -- tui --scenario sea_survival --locale en-US
```

You'll see a terminal dashboard: status gauges on the left, event log and AI panel in the middle, and available actions on the right. **Press `q` to quit.**

> The built-in fallback AI chooses randomly, so it won't survive long. Want a smarter agent? Read on.

### Let Claude Play

The best experience is to let Claude (or any MCP-capable agent) play the game. You need three terminals:

**Terminal A** — Start the game server:

```bash
cargo run -- serve --scenario sea_survival --locale en-US --socket data/waves.sock
```

**Terminal B** — Configure Claude Code's MCP connection:

```bash
# In Claude Code, add the MCP server:
/claude mcp add waves -- cargo run -- mcp --connect data/waves.sock

# Or edit ~/.claude/settings.json:
# "mcpServers": {
#   "waves": {
#     "command": "cargo",
#     "args": ["run", "--", "mcp", "--connect", "data/waves.sock"]
#   }
# }
```

Then tell Claude:

```
Start playing the sea_survival scenario. Use waves_start_run to begin,
then waves_step to advance. When a pending_decision appears, analyze the
state and choose the best action. Tell me your reasoning for each decision.
```

**Terminal C** — Watch the agent play:

```bash
cargo run -- tui --connect data/waves.sock
```

How they connect:

```
Terminal A: serve     ← owns the game
Terminal B: mcp       ← agent operates the game via MCP tools
Terminal C: tui       ← you watch the game
```

> 📖 See [Agent Playbook](docs/AGENT_PLAYBOOK.md) for the agent-facing guide.

### TUI Layout

```
┌─────────────────────────────────────────────────────┐
│ Waves Observer · Sea Survival           Day 3  ☀️     │
├──────────────┬──────────────────┬───────────────────┤
│ Status       │ AI Panel          │ Actions           │
│ HP    ████░  │ Goal: survive     │ 🎣 Fish           │
│ Hung  ██░░░  │ Worry: low water  │ 🍽️ Eat            │
│ Thst  ████░  │                   │ 💧 Collect rain    │
│ Enrg  ███░░  │ Event log         │ 🪵 Salvage         │
│              │ [Day 1] Storm!    │ 🔧 Repair raft     │
│ Resources    │ [Day 2] Fish shoal│ 😴 Rest            │
│ Food  1.8d   │ [Day 3] Floating  │ 🌤️ Observe weather │
│ Water 0.5d   │   crate found     │ 🗺️ Study chart     │
│ Wood  8      │                   │ 🧭 Change course   │
│              │ Activity log      │                   │
│ Environment  │ → Fished, got +2  │                   │
│ Clear · Calm  │ → Ate -1 food     │                   │
│ Dist  120km  │ → Salvaged +3 wood│                   │
├──────────────┴──────────────────┴───────────────────┤
│ q/Esc quit  |  p/Space pause                         │
└─────────────────────────────────────────────────────┘
```

### Scenarios

| Scenario | Description | Status |
|----------|-------------|--------|
| `sea_survival` | Ocean survival — manage HP, hunger, thirst, energy; survive weather and events; reach land | ✅ Playable |
| `desert_outpost` | Desert outpost (planned) | 🚧 Placeholder |

Scenario data (stats, resources, actions, events, balance, copy) is CSV-driven. Modify without touching code. See `scenarios/`.

### How It Works

- **Time advances** each tick, stats decay automatically
- **Random events** occur: storms, fish shoals, floating crates — agent must decide
- **Decision moments**: when `pending_decision` appears, the game pauses for agent input
- **Action narrowing**: available actions change based on current event and urgency
- **Win/lose**: HP 0 or raft destroyed = lose; reach land = win

### MCP Tools

Seven tools exposed to the agent:

```text
waves_start_run     Start a new game
waves_get_state     Get current state (compact, no full history)
waves_step          Advance one tick
waves_get_pending_decision  Check for pending decision
waves_submit_decision       Submit an action
waves_pause         Pause the game
waves_resume        Resume the game
```

Agent loop: `start → step → [pending? → submit → step → …]`

### Development

```bash
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test

cargo run -- validate scenario sea_survival
cargo run -- inspect config sea_survival
```

```text
src/core/          game engine — ticks, events, decisions, resolution
src/scenario/      Scenario trait and built-in scenarios
src/daemon/        shared local daemon and socket client
src/mcp/           MCP stdio server
src/tui/           Ratatui terminal UI
src/persistence/   SQLite persistence and replay
src/i18n/          localization catalog
scenarios/         scenario CSV data and locale files
tests/             integration tests
```

More details in [docs/](docs/).
