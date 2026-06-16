# Persistence And Replay

## 持久化方案

第一版采用：

```text
SQLite WAL + event sourcing + periodic snapshot
```

JSONL 作为导出和调试格式，不作为主存储。

目录结构建议：

```text
data/
  waves.sqlite
exports/
  run_xxx.domain_events.jsonl
  run_xxx.decisions.jsonl
  run_xxx.logs.jsonl
```

## 为什么使用 SQLite

```text
本地单机应用足够
事务可靠，异常退出后更容易恢复
查询方便，适合回放和复盘
WAL 模式下读写体验更好
部署成本低，不需要独立数据库服务
```

不采用 Postgres 的原因：

```text
本地 TUI 项目过重
安装和运维成本不值得
MVP 没有多人协作和远程查询需求
```

不采用纯 JSONL 的原因：

```text
查询和索引弱
事务能力弱
迁移困难
恢复和局部读取不方便
```

## 数据模型

建议表：

```text
runs
scenario_versions
snapshots
domain_events
decisions
logs
ui_events
```

## runs

记录一次运行。

```text
id
scenario_id
scenario_version
started_at
ended_at
status
seed
model
config_json
```

当前 `model` 字段记录为 `external-agent`，表示决策来自外部 MCP agent，而不是 runtime 内置模型。

## snapshots

周期性保存世界状态，便于快速恢复。

```text
id
run_id
tick
created_at
state_json
memory_json
```

建议策略：

```text
每 N 个 tick 保存一次
重大事件后保存一次
暂停或退出前保存一次
```

## domain_events

追加写入真实发生的系统事实。

```text
id
run_id
tick
event_type
created_at
payload_json
```

DomainEvent 是回放的核心事实来源。

## decisions

记录 agent 决策。

```text
id
run_id
tick
event_id
prompt_json
raw_output
parsed_json
choice
reason
risk_attitude
source
parse_status
error
created_at
```

`source` 可取：

```text
agent
fallback
```

## logs

保存人类可读日志。

```text
id
run_id
tick
level
title
body
created_at
source_event_id
```

## ui_events

用于调试 TUI 动效和回放 UI。

```text
id
run_id
tick
ui_event_type
target
payload_json
created_at
```

当前实现会持久化由 runtime 派生的 UiEvent，包括世界事件、agent 决策、数值变化、风险提示和日志提示。UiEvent 是表现层事件，不参与规则结算。

## 回放原则

确定性回放依赖：

```text
固定 RNG seed
固定 tick delta
保存 agent 决策结果
保存 agent 提交的行动和理由
保存 scenario version
从 snapshot + domain_events 恢复
```

回放流程：

```text
读取最近 snapshot
按 tick 顺序读取 domain_events
重建 WorldState
重放 decisions 和 logs
映射为 UiEvent
渲染 TUI 或导出报告
```

当前已实现摘要级 replay：

```bash
cargo run -- replay --run-id <run_id> --db data/waves.sqlite
```

输出内容包括 run 元数据、最近 snapshot tick、outcome 和各表计数。replay 命令只读取 SQLite，不调用外部 agent，也不推进 runtime。

后续 replay 增强：

```text
从 snapshot + domain_events 重建完整 WorldState 时间线
按 tick 回放 decisions、logs、ui_events
支持 TUI replay 播放和 JSONL 导出
```

## 导出

支持将一次 run 导出为 JSONL：

```text
domain_events.jsonl
decisions.jsonl
logs.jsonl
```

导出用于调试、分享、agent 行为分析和后续数据整理。
