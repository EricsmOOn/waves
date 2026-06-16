use waves::app::inspect_config;
use waves::config::{load_scenario_config, validate_config};
use waves::i18n::Catalog;

#[test]
fn sea_survival_config_is_valid() {
    let config = load_scenario_config("sea_survival").expect("config should load");
    let errors = validate_config(&config);
    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(config.manifest.id, "sea_survival");
    assert!(config.config_hash.len() >= 16);
}

#[test]
fn desert_outpost_config_is_valid() {
    let config = load_scenario_config("desert_outpost").expect("config should load");
    let errors = validate_config(&config);
    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(config.manifest.id, "desert_outpost");
    assert_eq!(config.manifest.entry, "desert_outpost");
    assert!(config.config_hash.len() >= 16);
}

#[test]
fn catalog_falls_back_to_default_locale_then_key() {
    let config = load_scenario_config("sea_survival").expect("config should load");
    let catalog = Catalog::new(
        "missing-locale",
        config.manifest.default_locale.clone(),
        config.locales.clone(),
    );

    assert_eq!(catalog.text("action.fish"), "捕鱼");
    assert_eq!(catalog.text("missing.key"), "missing.key");
}

#[test]
fn validation_errors_include_table_row_and_column() {
    let mut config = load_scenario_config("sea_survival").expect("config should load");
    config.tables.actions[0].name_key = "missing.action.name".to_string();
    config.tables.actions[1].id = config.tables.actions[0].id.clone();

    let errors = validate_config(&config);
    let rendered = errors.iter().map(ToString::to_string).collect::<Vec<_>>();

    assert!(
        rendered
            .iter()
            .any(|error| error.contains("actions.csv:2:name_key missing locale key"))
    );
    assert!(
        rendered
            .iter()
            .any(|error| error.contains("actions.csv:3:id duplicate id"))
    );
}

#[test]
fn inspect_config_summarizes_loaded_tables() {
    let inspection = inspect_config("sea_survival").expect("inspection should load");
    let lines = inspection.lines();

    assert!(lines.iter().any(|line| line == "scenario: sea_survival"));
    assert!(lines.iter().any(|line| line == "version: 0.1.0"));
    assert!(lines.iter().any(|line| line.contains("actions=9")));
    assert!(lines.iter().any(|line| line.contains("enabled_actions=9")));
    assert!(lines.iter().any(|line| line.contains("locales: en-US=")));
}

#[test]
fn inspect_config_summarizes_desert_outpost() {
    let inspection = inspect_config("desert_outpost").expect("inspection should load");
    let lines = inspection.lines();

    assert!(lines.iter().any(|line| line == "scenario: desert_outpost"));
    assert!(lines.iter().any(|line| line == "version: 0.1.0"));
    assert!(lines.iter().any(|line| line.contains("actions=9")));
    assert!(lines.iter().any(|line| line.contains("enabled_actions=9")));
    assert!(lines.iter().any(|line| line.contains("balance_keys=")));
}
