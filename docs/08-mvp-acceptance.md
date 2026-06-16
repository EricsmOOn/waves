# MVP Acceptance

## MVP 验收标准

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
沙漠前哨可以作为第二个配置化应用完成 smoke run
文本、玩法参数和数值主要从配置表加载
配置表修改后可以通过校验命令发现错误
中文 TUI 排版不会因中英文混排破坏对齐
```

## 第一版暂不实现

```text
网络前后端分离
Web UI
多人观察
远程同步
复杂插件市场
复杂地图
复杂 crafting
多个 agent 同时运行
大型世界观
长篇叙事生成
多 scenario 热加载
```

## MVP 工程里程碑

```text
1. Rust 项目骨架和配置加载
2. core runtime：clock、tick、event bus、pause/resume
3. scenario trait 和 sea_survival module
4. pending decision 和外部决策提交 API
5. sea_survival 状态、行动、事件、结算
6. SQLite schema、event append、snapshot
7. Ratatui 基础布局
8. UiEvent 和数值变化 UX
9. MCP stdio server
10. replay 和 deterministic tests
11. locale catalog 和中文宽度处理
12. scenario 配置表校验命令
13. scenario factory 支持 sea_survival 和 desert_outpost
```

## 验收用例

```text
启动应用后进入 TUI
选择 sea_survival scenario
使用 scripted runner 跑满 1 个游戏日
选择 desert_outpost scenario
使用 scripted runner 跑满 8 个 tick
状态随 tick 自动变化
至少触发 3 个事件
至少完成 3 次 agent 决策
非法 action submit 不改变状态
SQLite 中保存 run、events、decisions、logs
退出后可以从 snapshot 恢复
同 seed 回放结果一致
将一个 action 名称从配置表改名后，TUI 和 MCP pending decision 同步使用新文本
中文、英文、中英混排日志在固定宽度面板内正确换行
```

## 后续扩展方向

```text
scenario 脚本化
Web dashboard
远程观测
多模型对比运行
agent 行为分析报告
运行录像和分享
模型策略评分
```
