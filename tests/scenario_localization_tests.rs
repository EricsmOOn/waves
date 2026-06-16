use rand::SeedableRng;
use waves::config::load_scenario_config;
use waves::core::WorldEvent;
use waves::i18n::Catalog;
use waves::scenario::Scenario;
use waves::scenario::desert_outpost::DesertOutpostScenario;

#[test]
fn desert_outpost_resolution_summary_uses_desert_locale_terms() {
    let config = load_scenario_config("desert_outpost").expect("config should load");
    let catalog = Catalog::new(
        "zh-CN",
        config.manifest.default_locale.clone(),
        config.locales.clone(),
    );
    let scenario = DesertOutpostScenario::new(config);
    let mut state = scenario.initial_state();
    state.stats.raft = 40.0;
    let event = WorldEvent {
        id: "hull_damage".to_string(),
        title_key: "event.hull_damage.title".to_string(),
        severity: "warning".to_string(),
        resolver_id: "hull_damage_basic".to_string(),
    };
    let mut rng = rand::rngs::StdRng::seed_from_u64(1);

    let resolution = scenario.resolve_action(&mut state, &event, "repair_raft", &catalog, &mut rng);

    assert!(resolution.summary.contains("掩体"));
    assert!(!resolution.summary.contains("木筏"));
}
