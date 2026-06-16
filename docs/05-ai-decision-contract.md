# Agent Decision Contract

## 决策原则

事件触发时，runtime 生成一个 `PendingDecision`，暂停在决策点。外部 Codex/Claude 类 agent 通过 MCP 读取结构化状态和有限行动集合，只能选择其中一个行动并说明理由，不能决定行动结果，也不能直接修改世界状态。

`PendingDecision` 包含：

```text
decision id
scenario id
当前 tick
当前状态快照
当前事件
可选行动列表
```

## MCP 决策流程

在人类旁观模式下，MCP server 通常以 `waves mcp --connect data/waves.sock` 运行，并把工具调用转发给 `waves serve` 持有的共享 run。

```text
waves_start_run
waves_step
  -> 如果触发事件，返回 pending_decision
agent 与人类在聊天中讨论策略
waves_submit_decision
  -> runtime 校验 action_id 和 reason
  -> scenario 按规则结算
  -> runtime 写入 decisions/domain_events/logs/ui_events
```

`waves_step` 遇到 pending decision 会提前停止。这不是错误，而是游戏等待 agent 行动。

MCP 返回的是压缩后的决策视图：当前状态、当前 pending decision、最近日志、最近决策和本次结算摘要。完整 UI event 历史留给 TUI 和 replay，不直接塞进 agent 工具响应。

## 提交格式

agent 通过 `waves_submit_decision` 提交：

```json
{
  "decision_id": "tick-4-heat",
  "action_id": "rest",
  "reason": "高温正在消耗体力，先休息能保持后续行动成功率。",
  "risk_attitude": "cautious"
}
```

`risk_attitude` 是记录字段，不参与规则结算。规则结算只依赖当前状态、事件、行动、配置数值和 RNG。

## 校验异常

以下情况不改变世界状态：

```text
没有 active session
没有 pending decision
decision_id 与当前 pending decision 不匹配
action_id 不在可选行动中
reason 为空
run 已经结束
```

MCP 工具调用会返回结构化错误。外部 agent 可以重新查询 pending decision 后再次提交。

## 结算原则

agent 负责选择行动，系统负责结算结果。

结算输入：

```text
agent 选择的行动
agent 给出的理由
当前状态
当前资源
环境变量
行动基础成功率
行动风险修正
随机数
```

海上求生示例：

```text
行动：靠近木箱
基础成功率：60%
海况 Rough：-20%
体力低于 40：-10%
船体低于 80：-5%
最终成功率：25%
```

结算结果示例：

```text
失败
船体 -8
体力 -12
获得资源 0
触发日志：agent 误判了海浪
```

## 日志生成原则

日志用于把系统战报转译成简短、可读的运行记录。

日志输入：

```text
agent 选择了什么
agent 为什么选择
系统结算结果
状态变化
是否触发特殊事件
```

日志风格：

```text
简短
具体
围绕状态和后果
避免长篇叙事
避免让日志改写事实
```

当前实现使用规则模板生成日志，不调用模型。模型调用和模型服务配置都属于外部 agent 的职责，不进入 Waves runtime。
