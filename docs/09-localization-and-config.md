# Localization And Configuration

## 目标

多语言和配置化是一等能力。框架应让策划主要通过配置表调整文本、玩法和数值，而不是修改 Rust 代码。

核心目标：

```text
支持 zh-CN 和 en-US 起步
TUI 正确处理中英文混排
所有玩家可见文本从 locale key 获取
状态、资源、行动、事件、权重和数值从配置表加载
配置表对策划友好，可用表格软件编辑
启动前可校验配置错误
运行记录保存 scenario version 和配置摘要
```

## 多语言策略

第一版采用 locale table，不直接把文本写进代码。

目录：

```text
scenarios/{scenario_id}/locales/
  zh-CN.csv
  en-US.csv
```

CSV 格式：

```csv
key,text,notes
panel.status,状态,TUI 状态面板标题
panel.resources,资源,TUI 资源面板标题
action.fish,捕鱼,行动名
event.floating_crate.title,发现漂浮木箱,事件标题
log.salvage.failed,{actor} 打捞失败，木筏受损 {raft_delta}。,日志模板
```

规则：

```text
key 全局唯一
text 可包含变量占位符
notes 给策划和翻译使用，运行时忽略
缺失当前语言 key 时 fallback 到默认语言
默认语言也缺失时显示 key，并记录 warning
```

## TUI 中文排版

TUI 布局必须按终端显示宽度计算，而不是按字符数计算。

实现要求：

```text
使用 display width 计算宽度
使用 grapheme cluster 安全截断
wrap 支持无空格中文自然换行
英文优先按单词换行
中英文混排按实际显示宽度对齐
固定数值列和弹性文本列分开处理
上浮提示和日志文本先本地化，再测量宽度
```

建议封装统一文本工具：

```text
tui::text_width::display_width(text)
tui::text_width::truncate_to_width(text, width)
tui::text_width::wrap_to_width(text, width)
tui::text_width::pad_to_width(text, width, align)
```

验收样例：

```text
Water +0.4
淡水 +0.4
Raft 耐久 -8
发现 floating crate
AI 判断海浪较大，选择先观察 10 分钟。
```

这些字符串在同一面板内应正确对齐、换行和截断。

## 配置表工作流

策划友好的工作方式：

```text
策划在表格软件中编辑 CSV/TSV
开发维护 schema 和 resolver_id
提交前运行 waves validate scenario sea_survival
需要查看当前配置摘要时运行 waves inspect config sea_survival
校验通过后应用可加载配置
运行时记录配置版本和摘要
```

推荐格式：

```text
scenario.toml：应用 manifest、默认语言、表路径、版本
CSV/TSV：大部分策划配置表
locale CSV：本地化文本
Rust resolver：复杂结算函数
```

## 配置目录结构

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
    prompts.csv
  locales/
    zh-CN.csv
    en-US.csv
```

## scenario.toml

```toml
id = "sea_survival"
version = "0.1.0"
default_locale = "zh-CN"
entry = "sea_survival"

[tables]
stats = "tables/stats.csv"
resources = "tables/resources.csv"
actions = "tables/actions.csv"
events = "tables/events.csv"
event_weights = "tables/event_weights.csv"
balance = "tables/balance.csv"
panels = "tables/panels.csv"
prompts = "tables/prompts.csv"

[locales]
zh_CN = "locales/zh-CN.csv"
en_US = "locales/en-US.csv"
```

## 表职责

```text
stats.csv：状态 id、上下限、默认值、显示格式、排序
resources.csv：资源 id、默认值、单位、显示格式、排序
actions.csv：行动 id、消耗、收益、风险、前置条件、resolver_id
events.csv：事件 id、标题 key、描述 key、严重度、冷却、resolver_id
event_weights.csv：事件权重、环境修正、状态阈值修正
balance.csv：tick 消耗、成功率修正、天气修正、全局常量
panels.csv：TUI 面板、字段顺序、显示层级、可见性
prompts.csv：AI prompt 模板 key 和变量说明
```

## 表设计原则

```text
所有行必须有稳定 id
所有玩家可见文本只放 locale key
数值列使用明确单位
布尔值使用 true/false
枚举值必须在 schema 中声明
引用其他表时使用 id
复杂规则使用 resolver_id 绑定 Rust 函数
配置表不直接写任意代码
```

## 公式与规则

MVP 阶段优先使用表格列和 resolver_id。

```text
适合配置表：基础数值、权重、冷却、阈值、显示文本、排序
适合 Rust resolver：复杂随机结算、多状态联动、特殊事件链
```

当前 sea_survival 的行动成功率、成功/失败收益、距离变化、休息恢复、风险修正等数值放在 `balance.csv`。Rust resolver 负责读取这些 key 并处理状态联动。

后续可以引入受限表达式 DSL，但需要满足：

```text
只允许白名单函数
无文件和网络访问
可静态校验变量引用
错误能定位到表、行、列
```

## 校验命令

建议提供：

```text
waves validate scenario sea_survival
waves inspect config sea_survival
```

当前已实现：

```bash
cargo run -- validate scenario sea_survival
cargo run -- inspect config sea_survival
```

校验内容：

```text
必填字段存在
id 不重复
引用 id 存在
locale key 存在
数值在允许范围内
枚举值合法
resolver_id 已注册
prompt 变量占位符完整
表头没有拼写错误
```

错误示例：

```text
actions.csv:4:cost_energy repair_raft costs must be >= 0
events.csv:7:resolver_id resolver_id "storm_v3" is not registered
actions.csv:2:name_key missing locale key "action.repair_raft"
prompts.csv:2:template_key missing locale key "prompt.decision"
```

`inspect config` 输出示例：

```text
scenario: sea_survival
version: 0.1.0
default_locale: zh-CN
config_hash: 5315bd56ec223102
tables: stats=6 resources=5 actions=9 enabled_actions=9 events=8 panels=8 prompts=1 balance_keys=66
locales: en-US=80 zh-CN=80
resolvers: actions=9 events=8
```

## 运行时记录

每次运行需要保存：

```text
scenario_id
scenario_version
default_locale
active_locale
config_hash
loaded_table_versions
```

这样回放时可以知道当时使用的是哪一版配置，避免数值表更新后破坏旧运行的复盘。
