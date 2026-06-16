# Sea Survival Scenario

## 定位

海上求生是框架的首个应用示例，用于验证框架能否承载一个持续运行、规则结算、agent 自主决策、TUI 可观察的场景。

当前仓库还包含 `desert_outpost` 作为第二个配置化应用。它复用 survival resolver，重点验证 runtime、TUI、配置、本地化和持久化已经按 scenario 装配。海上求生仍然是第一套完整玩法样板。

选择海上求生的原因：

```text
目标清晰
资源压力直接
行动代价明显
少量规则即可产生可观察故事
适合展示不同模型的策略差异
```

## 核心目标

```text
活下去
寻找陆地
保存木筏
管理食物和淡水
在风险和收益之间取舍
```

## 典型决策

```text
今天捕鱼，还是修船
雨来了，优先收集淡水，还是休息恢复体力
发现漂浮木箱，要冒险靠近，还是保存木筏
风暴靠近，要绕开，还是趁风前进
远处出现岛影，要改变航向，还是先补充资源
```

## MVP 范围

第一版围绕：

```text
一个外部 agent
一艘木筏
一片海
少量状态
少量行动
少量事件
现实时间运行
agent 自主决策
规则结算结果
```

## 核心状态

```text
HP
Hunger
Thirst
Energy
Morale
Raft Durability
```

## 核心资源

```text
Food
Water
Wood
Fiber
Tool
```

## 环境状态

```text
Weather
Sea Condition
Wind
Time
Distance to Land
Risk Level
```

## Agent 内部状态

```text
Current Goal
Risk Bias
Recent Failure
Recent Success
Personality Tendency
Memory
```

MVP 性格参数控制在 4 个：

```text
risk_bias
water_priority
exploration_bias
repair_priority
```

## 行动集合

第一版行动控制在 9 个左右，每个行动都需要定义消耗、收益、风险、前置条件和可能触发事件。运行时会按当前事件和危急状态过滤可选行动。

```text
捕鱼
进食
收集雨水
打捞漂浮物
修理木筏
休息
观察天气
研究海图
改变航向
```

行动示例：

```text
捕鱼
消耗：体力
收益：食物
风险：低
受天气影响：是
可能事件：鱼钩损坏 / 捕到大鱼 / 一无所获
```

```text
打捞漂浮物
消耗：体力、时间
收益：木材、纤维、工具、未知物
风险：中
受海况影响：强
可能事件：落水 / 捡到补给 / 船体受损
```

```text
修理木筏
消耗：木材、纤维、体力
收益：船体耐久上升
风险：低
限制：需要材料
```

```text
改变航向
消耗：体力、时间
收益：接近陆地或避开风暴
风险：取决于天气、海况和导航信息
可能事件：偏航 / 发现岛影 / 遭遇恶劣海况
```

## 事件集合

第一版事件控制在少量高价值类型。

```text
下雨
暴晒
风暴
鱼群
漂浮木箱
船体损坏
远处岛影
废弃船只
```

事件作用：

```text
天气事件影响淡水、体力、航行和风险
资源事件提供可争取资源
损坏事件制造紧迫感
探索事件推动抵达陆地进度
身体事件迫使 agent 改变优先级
心理事件影响士气和性格参数
```

## 性格成长

agent 需要保留历史影响，形成可观察的行为变化。

可用性格参数：

```text
风险倾向
资源焦虑
探索欲
保守程度
目标执着
失败阴影
成功自信
```

更新规则示例：

```text
连续 3 次冒险失败：
风险倾向 -10
保守程度 +15

连续 2 次冒险成功：
风险倾向 +10
探索欲 +10

因缺水接近死亡：
资源焦虑 +20
淡水优先级 +30

成功接近陆地：
目标执着 +10
```

## Agent 控制

人类主要通过观察和聊天获得体验。游戏行动由外部 agent 通过 MCP 提交，runtime 只接受当前 `PendingDecision` 中列出的 action id。

TUI 只保留本地观察控制：

```text
暂停 / 继续
退出
```

## 成败条件

失败条件：

```text
HP 归零
脱水导致死亡
饥饿导致死亡
木筏耐久归零
```

阶段目标：

```text
存活 1 天
存活 3 天
存活 7 天
发现陆地线索
接近陆地
成功抵达岛屿或发出求救信号
```

第一版可以将“存活 7 天”作为基础验证目标，将“抵达岛屿”作为长期目标。

## 配置表拆分

海上求生示例应用的文本、玩法参数和数值平衡应从配置表获取。

建议目录：

```text
scenarios/sea_survival/
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
```

第二个应用采用相同目录结构：

```text
scenarios/desert_outpost/
  scenario.toml
  tables/
  locales/
```

新增应用时先复制目录结构，替换 `scenario.toml` 的 `id`、`entry`、表格和 locale 文本，再运行：

```bash
cargo run -- validate scenario <scenario_id>
cargo run -- inspect config <scenario_id>
cargo run -- run --scenario <scenario_id> --locale zh-CN --ticks 8
```

配置原则：

```text
状态和资源由表定义 id、默认值、上下限、显示顺序
行动由表定义消耗、收益、风险、前置条件和 resolver_id
事件由表定义触发条件、权重、可选行动和文案 key
数值平衡由表定义全局参数，例如 tick 消耗、天气修正、风险修正
所有面向玩家的文本使用 locale key
Rust 规则代码只处理复杂结算和框架逻辑
```

`actions.csv` 示例：

```csv
id,name_key,risk,cost_energy,cost_wood,cost_fiber,reward_type,resolver_id,enabled
fish,action.fish,low,12,0,0,food,fish_basic,true
eat_food,action.eat_food,low,2,0,0,food,eat_food_basic,true
collect_rain,action.collect_rain,low,6,0,0,water,collect_rain_basic,true
salvage,action.salvage,medium,16,0,0,mixed,salvage_basic,true
repair_raft,action.repair_raft,low,10,3,2,raft,repair_basic,true
```

`events.csv` 示例：

```csv
id,title_key,severity,base_weight,cooldown_ticks,resolver_id
rain,event.rain.title,notice,12,4,rain_basic
heat,event.heat.title,warning,10,3,heat_basic
storm,event.storm.title,danger,4,8,storm_basic
floating_crate,event.floating_crate.title,notice,8,5,salvage_opportunity
```

`zh-CN.csv` 示例：

```csv
key,text
action.fish,捕鱼
action.eat_food,进食
action.collect_rain,收集雨水
action.salvage,打捞漂浮物
event.floating_crate.title,发现漂浮木箱
panel.status,状态
panel.resources,资源
```
