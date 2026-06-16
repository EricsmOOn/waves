# Docs Index

本目录将 PRD 拆成面向 Agent 的小文档。后续实现时，优先按任务读取对应文件，避免一次加载全部上下文。

## 推荐阅读顺序

```text
1. 01-product-overview.md
2. 02-framework-architecture.md
3. 03-technical-implementation.md
4. 04-tui-ux-spec.md
5. 05-ai-decision-contract.md
6. 06-persistence-and-replay.md
7. 07-sea-survival-scenario.md
8. 08-mvp-acceptance.md
9. 09-localization-and-config.md
10. AGENT_PLAYBOOK.md
```

## 文档职责

```text
01-product-overview.md
产品是什么、用户体验是什么、为什么值得做。

02-framework-architecture.md
框架和应用如何分层，运行循环是什么，模块边界是什么。

03-technical-implementation.md
最终技术栈、Rust 方案、是否前后端分离、工程结构建议。

04-tui-ux-spec.md
TUI 如何保持灵动，信息层级、动效、数值变化和危险提示如何统一。

05-ai-decision-contract.md
外部 agent 通过 MCP 提交行动的结构化契约、异常处理、日志生成原则。

06-persistence-and-replay.md
SQLite WAL、事件溯源、快照、运行记录、回放和导出。

07-sea-survival-scenario.md
第一个应用示例：海上求生的状态、资源、行动、事件、成败条件；记录 `desert_outpost` 作为第二个配置化应用的验证边界。

08-mvp-acceptance.md
MVP 范围、验收标准、暂不实现内容和后续扩展方向。

09-localization-and-config.md
多语言、中文 TUI 排版、配置表结构、策划工作流和校验规则。

AGENT_PLAYBOOK.md
外部 agent 通过 MCP 玩 Waves 时应遵循的工具循环、决策规则和策略提示。
```

## Agent 使用建议

```text
做核心运行时：读 02、03、06
做 TUI：读 04，再读 02 的事件流
做 MCP agent 控制：读 05，再读 03 的 daemon/MCP/CLI 说明
让外部 agent 实际玩游戏并让人旁观：读 AGENT_PLAYBOOK
做海上求生示例：读 07，再读 05
做配置表和本地化：读 09，再读 07
做测试和验收：读 08，再读相关模块文档
```
