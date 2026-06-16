use crate::config::{ActionRow, LoadedScenarioConfig};
use crate::core::{
    ActionOption, DomainEvent, Environment, Memory, Personality, Resolution, Resources, RiskLevel,
    SeaCondition, StateChange, Stats, Weather, WorldEvent, WorldState,
};
use crate::i18n::Catalog;
use crate::scenario::Scenario;
use rand::Rng;
use rand::rngs::StdRng;
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SeaSurvivalScenario {
    config: LoadedScenarioConfig,
    last_event_tick: HashMap<String, u64>,
}

impl SeaSurvivalScenario {
    pub fn new(config: LoadedScenarioConfig) -> Self {
        Self {
            config,
            last_event_tick: HashMap::new(),
        }
    }

    fn balance(&self, key: &str, fallback: f64) -> f64 {
        self.config
            .tables
            .balance
            .get(key)
            .copied()
            .unwrap_or(fallback)
    }

    fn action(&self, action_id: &str) -> Option<&ActionRow> {
        self.config
            .tables
            .actions
            .iter()
            .find(|action| action.id == action_id)
    }
}

impl Scenario for SeaSurvivalScenario {
    fn id(&self) -> &str {
        &self.config.manifest.id
    }

    fn initial_state(&self) -> WorldState {
        let stat = |id: &str| {
            self.config
                .tables
                .stats
                .iter()
                .find(|row| row.id == id)
                .map(|row| row.default)
                .unwrap_or(0.0)
        };
        let resource = |id: &str| {
            self.config
                .tables
                .resources
                .iter()
                .find(|row| row.id == id)
                .map(|row| row.default)
                .unwrap_or(0.0)
        };

        WorldState {
            tick: 0,
            stats: Stats {
                hp: stat("hp"),
                hunger: stat("hunger"),
                thirst: stat("thirst"),
                energy: stat("energy"),
                morale: stat("morale"),
                raft: stat("raft"),
            },
            resources: Resources {
                food: resource("food"),
                water: resource("water"),
                wood: resource("wood"),
                fiber: resource("fiber"),
                tool: resource("tool"),
            },
            environment: Environment {
                weather: Weather::Cloudy,
                sea: SeaCondition::Moderate,
                wind: "NE".to_string(),
                risk: RiskLevel::Medium,
                day: 1,
                minute_of_day: 0,
                distance_to_land: self.balance("starting_distance", 120.0),
            },
            personality: Personality {
                risk_bias: 0.0,
                water_priority: 20.0,
                exploration_bias: 10.0,
                repair_priority: 15.0,
            },
            memory: Memory {
                goal_key: "ai.goal.survive".to_string(),
                concern_key: "ai.concern.water".to_string(),
                recent: Vec::new(),
            },
            alive: true,
            outcome: None,
        }
    }

    fn apply_tick(&mut self, state: &mut WorldState) -> Vec<DomainEvent> {
        state.tick += 1;
        let tick_minutes = self.balance("tick_minutes", 30.0) as u32;
        state.environment.minute_of_day += tick_minutes;
        while state.environment.minute_of_day >= 24 * 60 {
            state.environment.day += 1;
            state.environment.minute_of_day -= 24 * 60;
        }

        let before = state.clone();
        state.stats.hunger = clamp(
            state.stats.hunger + self.balance("hunger_per_tick", 1.2)
                - state.resources.food.min(0.05),
        );
        state.stats.thirst = clamp(
            state.stats.thirst + self.balance("thirst_per_tick", 1.6)
                - state.resources.water.min(0.08),
        );
        state.stats.energy = clamp(state.stats.energy - self.balance("energy_per_tick", 0.8));
        state.resources.water =
            (state.resources.water - self.balance("water_per_tick", 0.035)).max(0.0);
        state.resources.food =
            (state.resources.food - self.balance("food_per_tick", 0.025)).max(0.0);

        if state.stats.hunger > 82.0 {
            state.stats.hp = clamp(state.stats.hp - 0.8);
        }
        if state.stats.thirst > 82.0 {
            state.stats.hp = clamp(state.stats.hp - 1.2);
        }
        state.environment.risk = risk_from_state(state);
        state.memory.concern_key = concern_from_state(state).to_string();

        if let Some(outcome) = self.outcome(state) {
            state.alive = false;
            state.outcome = Some(outcome);
        }

        vec![DomainEvent::new(
            state.tick,
            "tick",
            json!({
                "before": before,
                "after": state,
            }),
        )]
    }

    fn select_event(&mut self, state: &WorldState, rng: &mut StdRng) -> WorldEvent {
        let mut weighted = Vec::new();
        for row in &self.config.tables.events {
            let last_tick = self.last_event_tick.get(&row.id).copied().unwrap_or(0);
            if state.tick.saturating_sub(last_tick) < row.cooldown_ticks {
                continue;
            }
            let mut weight = row.base_weight;
            weight *= match (row.id.as_str(), state.environment.weather) {
                ("rain", Weather::Cloudy) => 1.6,
                ("heat", Weather::Clear) => 1.5,
                ("storm", Weather::Storm) => 1.8,
                _ => 1.0,
            };
            weight *= match (row.id.as_str(), state.environment.sea) {
                ("storm", SeaCondition::Rough) => 1.5,
                ("hull_damage", SeaCondition::Rough) => 1.4,
                ("fish_shoal", SeaCondition::Calm) => 1.3,
                _ => 1.0,
            };
            if row.id == "island_silhouette" && state.environment.distance_to_land > 65.0 {
                weight *= 0.25;
            }
            weighted.push((row, weight.max(0.0)));
        }

        let total: f64 = weighted.iter().map(|(_, weight)| *weight).sum();
        let mut roll = rng.gen_range(0.0..total.max(1.0));
        let selected = weighted
            .iter()
            .find_map(|(row, weight)| {
                if roll <= *weight {
                    Some(*row)
                } else {
                    roll -= *weight;
                    None
                }
            })
            .or_else(|| weighted.first().map(|(row, _)| *row))
            .expect("sea_survival must define at least one event");

        self.last_event_tick.insert(selected.id.clone(), state.tick);
        event_environment_effect(selected.id.as_str(), state);

        WorldEvent {
            id: selected.id.clone(),
            title_key: selected.title_key.clone(),
            severity: selected.severity.clone(),
            resolver_id: selected.resolver_id.clone(),
        }
    }

    fn available_actions(&self, state: &WorldState, event: &WorldEvent) -> Vec<ActionOption> {
        self.config
            .tables
            .actions
            .iter()
            .filter(|action| action.enabled)
            .filter(|action| self.action_fits_event(state, event, action.id.as_str()))
            .filter(|action| state.stats.energy >= action.cost_energy)
            .filter(|action| state.resources.wood >= action.cost_wood)
            .filter(|action| state.resources.fiber >= action.cost_fiber)
            .map(|action| ActionOption {
                id: action.id.clone(),
                name_key: action.name_key.clone(),
                risk: action.risk.clone(),
                resolver_id: action.resolver_id.clone(),
            })
            .collect()
    }

    fn resolve_action(
        &self,
        state: &mut WorldState,
        event: &WorldEvent,
        action_id: &str,
        catalog: &Catalog,
        rng: &mut StdRng,
    ) -> Resolution {
        let before = state.clone();
        let action = self.action(action_id);
        if let Some(action) = action {
            state.stats.energy = clamp(state.stats.energy - action.cost_energy);
            state.resources.wood = (state.resources.wood - action.cost_wood).max(0.0);
            state.resources.fiber = (state.resources.fiber - action.cost_fiber).max(0.0);
        }

        let success_chance = self.success_chance(state, event, action_id);
        let success = action_id == "eat_food" || rng.gen_bool(success_chance.clamp(0.05, 0.95));
        let summary = match action_id {
            "fish" => resolve_fish(
                state,
                success,
                catalog,
                self.balance("fish_food_gain", 0.75),
                self.balance("fish_morale_gain", 2.0),
                self.balance("fish_morale_loss", 2.0),
            ),
            "eat_food" => resolve_eat_food(
                state,
                catalog,
                self.balance("eat_food_cost", 0.35),
                self.balance("eat_food_hunger_relief", 16.0),
                self.balance("eat_food_thirst_cost", 2.0),
            ),
            "collect_rain" => resolve_collect_rain(
                state,
                success,
                catalog,
                self.balance("collect_rain_gain_rain", 0.9),
                self.balance("collect_rain_gain_other", 0.25),
                self.balance("collect_rain_thirst_relief_per_water", 9.0),
                self.balance("collect_rain_fail_gain", 0.05),
            ),
            "salvage" => resolve_salvage(
                state,
                success,
                event,
                catalog,
                self.balance("salvage_wood_gain", 2.0),
                self.balance("salvage_fiber_gain", 1.0),
                self.balance("salvage_tool_gain_abandoned_ship", 1.0),
                self.balance("salvage_morale_gain", 3.0),
                self.balance("salvage_raft_loss_fail", 8.0),
                self.balance("salvage_energy_loss_fail", 4.0),
            ),
            "repair_raft" => resolve_repair(
                state,
                success,
                catalog,
                self.balance("repair_raft_gain_success", 14.0),
                self.balance("repair_raft_gain_fail", 4.0),
            ),
            "rest" => resolve_rest(
                state,
                catalog,
                self.balance("rest_energy_gain", 18.0),
                self.balance("rest_morale_gain", 1.0),
                self.balance("rest_hunger_cost", 1.0),
                self.balance("rest_thirst_cost", 1.0),
            ),
            "observe_weather" => resolve_observe(
                state,
                catalog,
                self.balance("observe_energy_cost_extra", 1.0),
            ),
            "study_chart" => resolve_study_chart(
                state,
                success,
                catalog,
                self.balance("study_distance_gain_success", 7.0),
                self.balance("study_morale_loss_fail", 1.0),
            ),
            "change_course" => resolve_change_course(
                state,
                success,
                catalog,
                self.balance("change_course_distance_gain_success", 10.0),
                self.balance("change_course_distance_loss_fail", 3.0),
                self.balance("change_course_energy_loss_fail", 4.0),
            ),
            _ => catalog.text("resolution.default"),
        };
        state.environment.risk = risk_from_state(state);
        state.memory.concern_key = concern_from_state(state).to_string();
        state.memory.recent.push(summary.clone());
        if state.memory.recent.len() > 6 {
            state.memory.recent.remove(0);
        }

        if let Some(outcome) = self.outcome(state) {
            state.alive = false;
            state.outcome = Some(outcome);
        }

        Resolution {
            success,
            summary,
            changes: diff_state(&before, state, action_id),
        }
    }

    fn outcome(&self, state: &WorldState) -> Option<String> {
        if state.stats.hp <= 0.0 {
            Some("hp_zero".to_string())
        } else if state.stats.raft <= 0.0 {
            Some("raft_destroyed".to_string())
        } else if state.environment.distance_to_land <= self.balance("victory_distance", 0.0) {
            Some("reached_land".to_string())
        } else {
            None
        }
    }
}

fn event_environment_effect(event_id: &str, _state: &WorldState) {
    let _ = event_id;
}

impl SeaSurvivalScenario {
    fn action_fits_event(&self, state: &WorldState, event: &WorldEvent, action_id: &str) -> bool {
        match action_id {
            "fish" => {
                event.id == "fish_shoal"
                    || state.resources.food < self.balance("urgent_food_resource_threshold", 0.9)
                    || state.stats.hunger > self.balance("urgent_hunger_threshold", 70.0)
            }
            "eat_food" => {
                state.resources.food >= self.balance("eat_food_cost", 0.35)
                    && state.stats.hunger > self.balance("eat_food_hunger_threshold", 45.0)
            }
            "collect_rain" => {
                event.id == "rain"
                    || state.resources.water < self.balance("urgent_water_resource_threshold", 1.0)
                    || state.stats.thirst > self.balance("urgent_thirst_threshold", 70.0)
            }
            "salvage" => {
                matches!(event.id.as_str(), "floating_crate" | "abandoned_ship")
                    || state.resources.wood < self.balance("urgent_wood_threshold", 3.0)
                    || state.resources.fiber < self.balance("urgent_fiber_threshold", 2.0)
            }
            "repair_raft" => {
                event.id == "hull_damage"
                    || state.stats.raft < self.balance("repair_action_raft_threshold", 78.0)
            }
            "study_chart" => {
                event.id == "island_silhouette"
                    || state.stats.energy >= self.balance("navigation_energy_threshold", 45.0)
            }
            "change_course" => {
                event.id == "island_silhouette"
                    || (state.environment.risk == RiskLevel::Low
                        && state.stats.energy >= self.balance("navigation_energy_threshold", 45.0))
            }
            "observe_weather" => {
                state.environment.risk != RiskLevel::Low
                    || matches!(event.id.as_str(), "heat" | "storm")
            }
            "rest" => true,
            _ => true,
        }
    }

    fn success_chance(&self, state: &WorldState, event: &WorldEvent, action_id: &str) -> f64 {
        let mut chance = match action_id {
            "fish" => self.balance("success_fish", 0.62),
            "eat_food" => self.balance("success_eat_food", 1.0),
            "collect_rain" => self.balance("success_collect_rain", 0.78),
            "salvage" => self.balance("success_salvage", 0.55),
            "repair_raft" => self.balance("success_repair_raft", 0.9),
            "rest" => self.balance("success_rest", 0.98),
            "observe_weather" => self.balance("success_observe_weather", 0.88),
            "study_chart" => self.balance("success_study_chart", 0.66),
            "change_course" => self.balance("success_change_course", 0.58),
            _ => self.balance("success_default", 0.5),
        };

        if state.environment.sea == SeaCondition::Rough {
            chance += self.balance("success_modifier_rough_sea", -0.14);
        }
        if state.environment.weather == Weather::Storm {
            chance += self.balance("success_modifier_storm", -0.18);
        }
        if state.stats.energy < self.balance("low_energy_threshold", 35.0) {
            chance += self.balance("success_modifier_low_energy", -0.10);
        }
        if state.stats.raft < self.balance("weak_raft_threshold", 55.0)
            && matches!(action_id, "salvage" | "change_course")
        {
            chance += self.balance("success_modifier_weak_raft_action", -0.12);
        }
        if event.id == "fish_shoal" && action_id == "fish" {
            chance += self.balance("success_bonus_fish_shoal", 0.22);
        }
        if event.id == "rain" && action_id == "collect_rain" {
            chance += self.balance("success_bonus_rain_collect", 0.16);
        }
        if event.id == "island_silhouette" && matches!(action_id, "study_chart" | "change_course") {
            chance += self.balance("success_bonus_island_navigation", 0.18);
        }
        chance
    }
}

fn resolve_fish(
    state: &mut WorldState,
    success: bool,
    catalog: &Catalog,
    food_gain: f64,
    morale_gain: f64,
    morale_loss: f64,
) -> String {
    if success {
        state.resources.food += food_gain;
        state.stats.morale = clamp(state.stats.morale + morale_gain);
        catalog.format(
            "resolution.fish.success",
            &[
                ("resource", catalog.text("resource.food")),
                ("amount", format_days(food_gain)),
            ],
        )
    } else {
        state.stats.morale = clamp(state.stats.morale - morale_loss);
        catalog.format(
            "resolution.fish.failure",
            &[("stat", catalog.text("stat.morale"))],
        )
    }
}

fn resolve_eat_food(
    state: &mut WorldState,
    catalog: &Catalog,
    food_cost: f64,
    hunger_relief: f64,
    thirst_cost: f64,
) -> String {
    let consumed = state.resources.food.min(food_cost);
    let relief = if food_cost > 0.0 {
        hunger_relief * (consumed / food_cost)
    } else {
        hunger_relief
    };
    state.resources.food = (state.resources.food - consumed).max(0.0);
    state.stats.hunger = clamp(state.stats.hunger - relief);
    state.stats.thirst = clamp(state.stats.thirst + thirst_cost);
    catalog.format(
        "resolution.eat_food",
        &[
            ("stat", catalog.text("stat.hunger")),
            ("amount", format_int(relief)),
        ],
    )
}

fn resolve_collect_rain(
    state: &mut WorldState,
    success: bool,
    catalog: &Catalog,
    rain_gain: f64,
    other_gain: f64,
    thirst_relief_per_water: f64,
    fail_gain: f64,
) -> String {
    let gain = if state.environment.weather == Weather::Rain {
        rain_gain
    } else {
        other_gain
    };
    if success {
        state.resources.water += gain;
        state.stats.thirst = clamp(state.stats.thirst - gain * thirst_relief_per_water);
        catalog.format(
            "resolution.collect_water.success",
            &[
                ("resource", catalog.text("resource.water")),
                ("amount", format_days(gain)),
            ],
        )
    } else {
        state.resources.water += fail_gain;
        catalog.format(
            "resolution.collect_water.failure",
            &[("resource", catalog.text("resource.water"))],
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_salvage(
    state: &mut WorldState,
    success: bool,
    event: &WorldEvent,
    catalog: &Catalog,
    wood_gain: f64,
    fiber_gain: f64,
    abandoned_ship_tool_gain: f64,
    morale_gain: f64,
    raft_loss_fail: f64,
    energy_loss_fail: f64,
) -> String {
    if success {
        state.resources.wood += wood_gain;
        state.resources.fiber += fiber_gain;
        if event.id == "abandoned_ship" {
            state.resources.tool += abandoned_ship_tool_gain;
        }
        state.stats.morale = clamp(state.stats.morale + morale_gain);
        catalog.format(
            "resolution.salvage.success",
            &[
                ("wood", catalog.text("resource.wood")),
                ("wood_amount", format_int(wood_gain)),
                ("fiber", catalog.text("resource.fiber")),
                ("fiber_amount", format_int(fiber_gain)),
            ],
        )
    } else {
        state.stats.raft = clamp(state.stats.raft - raft_loss_fail);
        state.stats.energy = clamp(state.stats.energy - energy_loss_fail);
        catalog.format(
            "resolution.salvage.failure",
            &[
                ("action", catalog.text("action.salvage")),
                ("durability", catalog.text("stat.raft")),
                ("amount", format_int(raft_loss_fail)),
            ],
        )
    }
}

fn resolve_repair(
    state: &mut WorldState,
    success: bool,
    catalog: &Catalog,
    success_gain: f64,
    fail_gain: f64,
) -> String {
    if success {
        state.stats.raft = clamp(state.stats.raft + success_gain);
        catalog.format(
            "resolution.repair.success",
            &[
                ("durability", catalog.text("stat.raft")),
                ("amount", format_int(success_gain)),
            ],
        )
    } else {
        state.stats.raft = clamp(state.stats.raft + fail_gain);
        catalog.format(
            "resolution.repair.failure",
            &[
                ("durability", catalog.text("stat.raft")),
                ("amount", format_int(fail_gain)),
            ],
        )
    }
}

fn resolve_rest(
    state: &mut WorldState,
    catalog: &Catalog,
    energy_gain: f64,
    morale_gain: f64,
    hunger_cost: f64,
    thirst_cost: f64,
) -> String {
    state.stats.energy = clamp(state.stats.energy + energy_gain);
    state.stats.morale = clamp(state.stats.morale + morale_gain);
    state.stats.hunger = clamp(state.stats.hunger + hunger_cost);
    state.stats.thirst = clamp(state.stats.thirst + thirst_cost);
    catalog.format(
        "resolution.rest",
        &[
            ("stat", catalog.text("stat.energy")),
            ("amount", format_int(energy_gain)),
        ],
    )
}

fn resolve_observe(state: &mut WorldState, catalog: &Catalog, energy_cost_extra: f64) -> String {
    state.stats.energy = clamp(state.stats.energy - energy_cost_extra);
    if state.environment.risk == RiskLevel::High {
        state.personality.risk_bias -= 1.0;
    }
    catalog.text("resolution.observe")
}

fn resolve_study_chart(
    state: &mut WorldState,
    success: bool,
    catalog: &Catalog,
    distance_gain_success: f64,
    morale_loss_fail: f64,
) -> String {
    if success {
        state.environment.distance_to_land =
            (state.environment.distance_to_land - distance_gain_success).max(0.0);
        state.personality.exploration_bias += 1.0;
        catalog.format(
            "resolution.study_chart.success",
            &[
                ("target", catalog.text("nav.target")),
                ("amount", format_int(distance_gain_success)),
                ("unit", catalog.text("unit.distance")),
            ],
        )
    } else {
        state.stats.morale = clamp(state.stats.morale - morale_loss_fail);
        catalog.text("resolution.study_chart.failure")
    }
}

fn resolve_change_course(
    state: &mut WorldState,
    success: bool,
    catalog: &Catalog,
    distance_gain_success: f64,
    distance_loss_fail: f64,
    energy_loss_fail: f64,
) -> String {
    if success {
        state.environment.distance_to_land =
            (state.environment.distance_to_land - distance_gain_success).max(0.0);
        catalog.format(
            "resolution.change_course.success",
            &[
                ("target", catalog.text("nav.target")),
                ("amount", format_int(distance_gain_success)),
                ("unit", catalog.text("unit.distance")),
            ],
        )
    } else {
        state.environment.distance_to_land += distance_loss_fail;
        state.stats.energy = clamp(state.stats.energy - energy_loss_fail);
        catalog.format(
            "resolution.change_course.failure",
            &[
                ("target", catalog.text("nav.target")),
                ("amount", format_int(distance_loss_fail)),
                ("unit", catalog.text("unit.distance")),
            ],
        )
    }
}

fn format_days(value: f64) -> String {
    format!("{value:.2}")
}

fn format_int(value: f64) -> String {
    format!("{value:.0}")
}

fn risk_from_state(state: &WorldState) -> RiskLevel {
    if state.stats.hp < 25.0 || state.stats.raft < 25.0 || state.stats.thirst > 88.0 {
        RiskLevel::Critical
    } else if state.stats.raft < 45.0
        || state.resources.water < 0.35
        || state.environment.weather == Weather::Storm
    {
        RiskLevel::High
    } else if state.resources.water < 0.8
        || state.stats.energy < 35.0
        || state.environment.sea == SeaCondition::Rough
    {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    }
}

fn concern_from_state(state: &WorldState) -> &'static str {
    if state.resources.water < 0.7 || state.stats.thirst > 70.0 {
        "ai.concern.water"
    } else if state.stats.raft < 60.0 {
        "ai.concern.raft"
    } else if state.stats.energy < 35.0 {
        "ai.concern.energy"
    } else {
        "ai.concern.land"
    }
}

fn diff_state(before: &WorldState, after: &WorldState, reason: &str) -> Vec<StateChange> {
    let mut changes = Vec::new();
    push_change(&mut changes, "hp", before.stats.hp, after.stats.hp, reason);
    push_change(
        &mut changes,
        "hunger",
        before.stats.hunger,
        after.stats.hunger,
        reason,
    );
    push_change(
        &mut changes,
        "thirst",
        before.stats.thirst,
        after.stats.thirst,
        reason,
    );
    push_change(
        &mut changes,
        "energy",
        before.stats.energy,
        after.stats.energy,
        reason,
    );
    push_change(
        &mut changes,
        "morale",
        before.stats.morale,
        after.stats.morale,
        reason,
    );
    push_change(
        &mut changes,
        "raft",
        before.stats.raft,
        after.stats.raft,
        reason,
    );
    push_change(
        &mut changes,
        "food",
        before.resources.food,
        after.resources.food,
        reason,
    );
    push_change(
        &mut changes,
        "water",
        before.resources.water,
        after.resources.water,
        reason,
    );
    push_change(
        &mut changes,
        "wood",
        before.resources.wood,
        after.resources.wood,
        reason,
    );
    push_change(
        &mut changes,
        "fiber",
        before.resources.fiber,
        after.resources.fiber,
        reason,
    );
    push_change(
        &mut changes,
        "distance_to_land",
        before.environment.distance_to_land,
        after.environment.distance_to_land,
        reason,
    );
    changes
}

fn push_change(
    changes: &mut Vec<StateChange>,
    target: &str,
    before: f64,
    after: f64,
    reason: &str,
) {
    let delta = after - before;
    if delta.abs() >= 0.001 {
        changes.push(StateChange {
            target: target.to_string(),
            before,
            after,
            delta,
            reason: reason.to_string(),
        });
    }
}

fn clamp(value: f64) -> f64 {
    value.clamp(0.0, 100.0)
}
