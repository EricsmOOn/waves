use rand::SeedableRng;
use waves::app::run_headless;
use waves::config::load_scenario_config;
use waves::core::WorldEvent;
use waves::i18n::Catalog;
use waves::scenario::Scenario;
use waves::scenario::sea_survival::SeaSurvivalScenario;

#[test]
fn sea_survival_runs_one_day_with_scripted_agent() {
    let report = run_headless("sea_survival", "zh-CN", 48, 7, None).expect("headless run");

    assert_eq!(report.final_state.tick, 48);
    assert!(report.decisions >= 10);
    assert!(report.logs >= 20);
    assert!(report.domain_events >= 70);
    assert!(report.final_state.stats.hp > 0.0);
    assert!(report.final_state.stats.raft > 0.0);
}

#[test]
fn deterministic_seed_repeats_final_state() {
    let first = run_headless("sea_survival", "zh-CN", 24, 99, None).expect("first run");
    let second = run_headless("sea_survival", "zh-CN", 24, 99, None).expect("second run");

    assert_eq!(first.final_state, second.final_state);
}

#[test]
fn seed_42_scripted_run_survives_reported_thirst_spike() {
    let report = run_headless("sea_survival", "zh-CN", 120, 42, None).expect("headless run");

    assert_eq!(report.final_state.tick, 120);
    assert!(report.final_state.stats.hp > 0.0);
    assert!(report.final_state.stats.thirst < 100.0);
}

#[test]
fn balance_table_controls_rest_resolution_values() {
    let mut config = load_scenario_config("sea_survival").expect("config should load");
    config
        .tables
        .balance
        .insert("rest_energy_gain".to_string(), 30.0);
    let catalog = Catalog::new(
        "zh-CN",
        config.manifest.default_locale.clone(),
        config.locales.clone(),
    );
    let scenario = SeaSurvivalScenario::new(config);
    let mut state = scenario.initial_state();
    state.stats.energy = 20.0;
    let mut rng = rand::rngs::StdRng::seed_from_u64(1);
    let event = WorldEvent {
        id: "heat".to_string(),
        title_key: "event.heat.title".to_string(),
        severity: "warning".to_string(),
        resolver_id: "heat_basic".to_string(),
    };

    let resolution = scenario.resolve_action(&mut state, &event, "rest", &catalog, &mut rng);

    assert_eq!(state.stats.energy, 50.0);
    assert_eq!(resolution.summary, "体力 +30");
}
