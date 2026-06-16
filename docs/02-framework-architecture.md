# Framework Architecture

## 框架与应用关系

框架需要抽象出一套稳定的承载能力，让不同应用可以复用同一套运行机制。

框架层能力：

```text
时间推进
状态管理
事件调度
行动注册
规则结算
agent 决策点编排
结构化输出解析
记忆与性格参数更新
日志生成
TUI 展示与动效规则
MCP 外部控制接口
存档与回放
```

应用层配置：

```text
世界主题
核心状态
资源系统
环境变量
事件集合
行动集合
胜负条件
结算公式
提示词模板
日志文案风格
TUI 面板命名
本地化文本
数值平衡表
配置表 schema
```

框架目标是让新应用通过配置和少量规则代码接入，而不需要重写时间循环、agent 控制接口、TUI 展示、日志和历史记录。

## 架构形态

第一版核心代码仍是模块化单体，但可观察运行采用本地 daemon：

```text
waves serve
├─ core runtime
├─ scenario runtime
├─ persistence
└─ Unix socket JSON-lines RPC

waves mcp --connect <socket>
└─ MCP stdio bridge

waves tui --connect <socket>
└─ read-only TUI observer
```

`serve` 持有唯一 `RuntimeSession`，MCP bridge 和 TUI observer 都通过本地 socket 访问同一局游戏。MCP stdio 是外部工具 agent 的主控制面；TUI 不提交游戏行动，只负责观察、暂停、继续和退出。后续如果要做 Web UI 或远程观测，可以在 daemon 外增加 WebSocket 或 HTTP 层。

## 核心运行循环

```text
现实时间推进
↓
状态自然变化
↓
环境、资源、探索机会刷新
↓
事件出现
↓
系统整理当前状态和可选行动，生成 PendingDecision
↓
外部 agent 通过 MCP 读取状态并提交一个行动
↓
系统校验行动并根据规则结算行动结果
↓
状态、资源、风险、性格参数发生变化
↓
系统生成结构化战报
↓
TUI 更新状态、决策、日志
↓
人类在聊天中协作，并通过 TUI 观察
```

MVP 需要成立的三件事：

```text
状态持续变化
agent 决策可观察
结果由规则结算
```

## 时间机制

第一版采用现实时间制。框架会在不需要决策时持续运行，遇到事件后停在 pending decision 等待外部 agent，形成桌面常驻的观测感。

建议初始节奏：

```text
基础 tick：每 30 秒
小事件：约每 3-5 分钟
中事件：约每 15-30 分钟
重大事件：按生存天数、风险等级和探索进度触发
```

每个 tick 处理基础消耗和环境变化。事件触发时，系统生成 pending decision 并等待外部 agent 提交行动。

## 事件流

框架内部使用事件流连接核心模块。

```text
Runtime emits DomainEvent
Persistence stores DomainEvent
TUI maps DomainEvent to UiEvent
Replay reads DomainEvent and snapshots
Logger derives human-readable logs from BattleReport
```

DomainEvent 是事实记录，UiEvent 是表现层事件。TUI 动效不应反向影响世界状态。

## Scenario 接口

```rust
pub trait Scenario {
    fn id(&self) -> &str;
    fn initial_state(&self) -> WorldState;
    fn apply_tick(&mut self, state: &mut WorldState) -> Vec<DomainEvent>;
    fn select_event(&mut self, state: &WorldState, rng: &mut StdRng) -> WorldEvent;
    fn available_actions(&self, state: &WorldState, event: &WorldEvent) -> Vec<ActionOption>;
    fn resolve_action(
        &self,
        state: &mut WorldState,
        event: &WorldEvent,
        action_id: &str,
        rng: &mut StdRng,
    ) -> Resolution;
    fn outcome(&self, state: &WorldState) -> Option<String>;
}
```

当前通过 `scenario.toml` 的 `entry` 字段创建 scenario 实例：

```text
entry = "sea_survival" -> SeaSurvivalScenario
entry = "desert_outpost" -> DesertOutpostScenario
```

`RuntimeSession` 持有 `Box<dyn Scenario>`。框架层只依赖 trait，TUI 标题、文本和配置摘要从当前 scenario 配置读取。

第一版使用 Rust module 处理复杂规则入口，同时把文本、玩法参数、行动、事件、数值平衡和 TUI 面板配置放入配置表。当前 `desert_outpost` 复用 survival resolver，用于验证多应用装配路径；后续再按真实差异拆出更通用的状态模型和规则边界。

## 配置加载边界

框架启动时加载 scenario manifest，再加载对应数据表和本地化表。

```text
scenario manifest：声明应用 id、版本、入口、默认语言、表路径
data tables：定义状态、资源、行动、事件、权重、数值参数
locale tables：定义界面文本、日志模板、事件文案、行动文案
rule bindings：把配置表里的 resolver_id 绑定到 Rust 规则函数
schema validation：启动前校验字段、类型、引用和取值范围
```

复杂公式可以逐步引入表达式 DSL。MVP 阶段优先使用可读的数值列和 `resolver_id`，避免让策划表里出现难以调试的任意脚本。

当前内置应用：

```text
scenarios/sea_survival/      首个完整示例应用
scenarios/desert_outpost/    第二个配置化应用，用于验证框架抽象
```
