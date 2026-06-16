# Agent 自主决策观测框架 PRD

本项目是一套 TUI + MCP 形态的实时 agent 自主决策观测框架。开发者可以定义规则化世界，配置状态、事件、行动和结算规则；外部 Codex/Claude 类 agent 通过 MCP 玩游戏，人类一边聊天协作，一边通过 TUI 观察 agent 如何在真实时间流逝中自主决策、犯错、调整策略并形成可观察的行为模式。

## 文档导航

按维度拆分后的详细文档位于 `docs/`：

```text
docs/README.md                         文档索引与 Agent 阅读顺序
docs/01-product-overview.md            产品定位、核心体验、卖点
docs/02-framework-architecture.md      框架与应用关系、运行循环、模块边界
docs/03-technical-implementation.md    Rust/Ratatui/SQLite/MCP 技术方案
docs/04-tui-ux-spec.md                 TUI 信息层级、动效、数值变化 UX
docs/05-ai-decision-contract.md        agent 决策提交、校验、日志生成
docs/06-persistence-and-replay.md      SQLite WAL、事件溯源、快照、回放
docs/07-sea-survival-scenario.md       海上求生示例应用 MVP
docs/08-mvp-acceptance.md              MVP 范围、验收标准、后续扩展
docs/09-localization-and-config.md     多语言、中文 TUI 排版、配置表工作流
```

## 核心定义

```text
外部 agent 是决策主体
人类是协作者和观察者
框架是运行底座
应用是规则世界
TUI 是观察窗口
MCP 是 agent 控制接口
海上求生是首个示例应用
```

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
Architecture: local daemon + MCP bridge + TUI observer
Scenario: sea_survival as first application
Testing: cargo test + deterministic replay tests
```

## MVP 验收目标

```text
框架可以在 TUI 中持续运行
状态会随现实时间自动变化
事件会按规则出现
agent 会基于结构化状态选择行动
系统会根据规则结算结果
非法行动提交不会改变状态
人类能看到状态、决策、理由、结果和日志
人类能暂停、继续观察
一次运行中能形成可回看的决策历史
海上求生可以作为首个示例应用完整运行
文本、玩法参数和数值主要从配置表加载
中文 TUI 排版不会因中英文混排破坏对齐
```
