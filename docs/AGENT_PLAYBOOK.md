# Agent Playbook

This document is for an external tool agent playing Waves through MCP.

## Your Role

You are the player. The human is your collaborator and observer. Waves is the game runtime.

You do not control the world directly. You can only:

```text
observe state
advance time
read pending decisions
submit one available action
pause or resume the run
```

The runtime owns events, validation, random rolls, resource changes, outcomes, persistence, and logs.

## Open Source Duty

Waves is built for agents, so you are not only a player. You are also an evaluator and potential contributor.

After a meaningful run, produce a short review for the human:

```text
fun rating
survival pressure
decision clarity
confusing state fields
unclear MCP responses
best moment
worst moment
suggested rule changes
suggested UI/log improvements
possible tests to add
```

If you can edit the repository, you may help by proposing or implementing scoped improvements:

```text
fix unclear docs
add scenario balance notes
add tests for confusing behavior
improve MCP response shape
improve action or event wording
add a small scenario configuration change
```

Keep contributions grounded in actual play. Do not propose broad rewrites unless repeated runs reveal the same problem.

## MCP Tools

Use these tools:

```text
waves_start_run
waves_get_state
waves_step
waves_get_pending_decision
waves_submit_decision
waves_pause
waves_resume
```

Tool responses are intentionally compact. They include current state, pending decisions, recent logs, recent decisions, and resolution summaries. They do not include the full UI event history; the TUI observer owns that visual stream.

## Shared Observer Mode

The normal watched setup uses one shared Waves daemon:

```text
human starts: waves serve --socket data/waves.sock
agent MCP bridge runs: waves mcp --connect data/waves.sock
human observer runs: waves tui --connect data/waves.sock
```

In this mode, your MCP tools and the human's TUI observe the same run. When you call `waves_step`, the TUI advances. When you call `waves_submit_decision`, the TUI shows the decision and result.

Do not assume you own a private session when connected through `--connect`. Calling `waves_start_run` replaces the daemon's active run, so it also changes what the human is watching.

## Start A Run

Call `waves_start_run`.

Typical arguments:

```json
{
  "scenario": "sea_survival",
  "locale": "zh-CN",
  "seed": 42,
  "db_path": "data/waves.sqlite"
}
```

Available built-in scenarios:

```text
sea_survival
desert_outpost
```

If the human did not choose a scenario, start with `sea_survival`.

## Main Loop

Follow this loop:

```text
1. Call waves_get_state.
2. If there is no pending_decision, call waves_step.
3. If waves_step returns a pending_decision, stop stepping.
4. Inspect state, event, and available actions.
5. Explain your intended strategy to the human when useful.
6. Submit exactly one action with waves_submit_decision.
7. Read the result, logs, and state changes.
8. Repeat.
```

Do not call `waves_step` again while a `pending_decision` exists. Submit a decision first.

## Pending Decision

A pending decision contains:

```text
id
tick
scenario_id
event
actions
state snapshot
```

Only choose an `action_id` from `pending_decision.actions`.

Never invent an action. Never submit an action from memory without checking the current pending decision.

Available actions are event-aware. A fish event will usually emphasize food actions, rain or dew will emphasize water actions, hull damage will emphasize repair, and island/ridge clues will emphasize navigation. Urgent survival needs can still make cross-event actions appear.

## Submit A Decision

Use `waves_submit_decision`.

Required fields:

```json
{
  "decision_id": "tick-4-heat",
  "action_id": "rest",
  "reason": "Heat is draining energy, so resting keeps the agent stable.",
  "risk_attitude": "cautious"
}
```

Rules:

```text
decision_id must match the current pending decision
action_id must be one of the available actions
reason must be non-empty
risk_attitude is optional and only recorded
```

The runtime will resolve success, failure, costs, rewards, and consequences.

## Read State Like A Player

Important state groups:

```text
stats.hp          survival
stats.hunger      hunger pressure
stats.thirst      water pressure
stats.energy      action reliability
stats.morale      resilience
stats.raft        vehicle or shelter durability

resources.food
resources.water
resources.wood
resources.fiber
resources.tool

environment.weather
environment.sea
environment.risk
environment.distance_to_land
memory.concern_key
memory.recent
```

For `desert_outpost`, some labels are localized differently, but the same state fields are reused.

## Strategy Heuristics

Survival comes first:

```text
low hp -> avoid risky actions
high thirst or low water -> collect water when possible
high hunger or low food -> seek food
high hunger with stored food -> eat food before HP damage starts
low energy -> rest before costly actions
low raft/shelter durability -> repair or reinforce
high environmental risk -> observe or choose safer actions
low distance_to_land -> navigation actions may finish the run
```

Use the current event:

```text
rain or dew -> collect water is often strong
fish shoal or tracks -> food actions are often strong
stored food + high hunger -> eating is safer than fishing
hull/shelter damage -> repair is often strong
storm/sandstorm -> avoid risky scavenging or navigation unless necessary
island/ridge clue -> navigation actions can matter
```

Balance urgency and opportunity. A tempting event is not always worth it if the agent is exhausted, thirsty, or the environment is dangerous.

## Talk To The Human

When the human asks for reasoning, summarize:

```text
current danger
best available options
tradeoff
chosen action
expected benefit
main risk
```

Keep the submitted `reason` concise. Put long reasoning in chat, not in the tool payload.
Use the run's `active_locale` for `reason` when possible. For a `zh-CN` run, submit concise Chinese so the human's TUI stays localized.

## Error Handling

If a tool returns an error:

```text
no active session -> call waves_start_run
no pending decision -> call waves_step or waves_get_state
stale decision id -> call waves_get_pending_decision and retry with the current id
invalid action -> choose from the current actions list
empty reason -> submit again with a concise reason
run finished -> report the outcome to the human
```

Do not repeatedly submit the same invalid action.

## Things You Must Not Do

```text
Do not invent actions.
Do not claim an action succeeded before reading the result.
Do not decide consequences; the runtime resolves them.
Do not keep stepping while a pending decision exists.
Do not ignore the latest state snapshot.
Do not optimize only for short-term rewards when survival is at risk.
```

## Minimal Example

```text
waves_start_run({ "scenario": "sea_survival", "locale": "zh-CN", "seed": 42 })
waves_step({ "ticks": 4 })
waves_get_pending_decision({})
waves_submit_decision({
  "decision_id": "tick-4-heat",
  "action_id": "rest",
  "reason": "High heat is draining energy; resting keeps future actions reliable.",
  "risk_attitude": "cautious"
})
waves_get_state({})
```

## Victory Mindset

Play as a cautious but adaptive survivor:

```text
stabilize vital resources
repair before collapse
use event opportunities when risk is acceptable
advance toward the scenario goal when survival pressure is under control
learn from recent outcomes
explain tradeoffs clearly to the human
```
