use crate::config::LoadedScenarioConfig;
use std::collections::HashSet;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub location: String,
    pub row: Option<usize>,
    pub column: Option<String>,
    pub message: String,
}

impl ValidationError {
    fn new(location: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            location: location.into(),
            row: None,
            column: None,
            message: message.into(),
        }
    }

    fn at(
        location: impl Into<String>,
        row: usize,
        column: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            location: location.into(),
            row: Some(row),
            column: Some(column.into()),
            message: message.into(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.row, self.column.as_deref()) {
            (Some(row), Some(column)) => {
                write!(f, "{}:{}:{} {}", self.location, row, column, self.message)
            }
            (Some(row), None) => write!(f, "{}:{} {}", self.location, row, self.message),
            _ => write!(f, "{} {}", self.location, self.message),
        }
    }
}

impl std::error::Error for ValidationError {}

pub fn validate_config(config: &LoadedScenarioConfig) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if config.manifest.id.trim().is_empty() {
        errors.push(ValidationError::new("scenario.toml", "id is required"));
    }
    if config.manifest.version.trim().is_empty() {
        errors.push(ValidationError::new("scenario.toml", "version is required"));
    }
    if !config.locales.contains_key(&config.manifest.default_locale) {
        errors.push(ValidationError::new(
            "scenario.toml",
            format!(
                "default_locale {} has no locale table",
                config.manifest.default_locale
            ),
        ));
    }

    validate_ids(
        "stats.csv",
        config
            .tables
            .stats
            .iter()
            .enumerate()
            .map(|(index, row)| (csv_row(index), row.id.as_str())),
        &mut errors,
    );
    validate_ids(
        "resources.csv",
        config
            .tables
            .resources
            .iter()
            .enumerate()
            .map(|(index, row)| (csv_row(index), row.id.as_str())),
        &mut errors,
    );
    validate_ids(
        "actions.csv",
        config
            .tables
            .actions
            .iter()
            .enumerate()
            .map(|(index, row)| (csv_row(index), row.id.as_str())),
        &mut errors,
    );
    validate_ids(
        "events.csv",
        config
            .tables
            .events
            .iter()
            .enumerate()
            .map(|(index, row)| (csv_row(index), row.id.as_str())),
        &mut errors,
    );
    validate_ids(
        "panels.csv",
        config
            .tables
            .panels
            .iter()
            .enumerate()
            .map(|(index, row)| (csv_row(index), row.id.as_str())),
        &mut errors,
    );

    let default_locale = config
        .locales
        .get(&config.manifest.default_locale)
        .map(|table| &table.entries);

    for (index, row) in config.tables.stats.iter().enumerate() {
        let csv_row = csv_row(index);
        if row.min > row.max {
            errors.push(ValidationError::at(
                "stats.csv",
                csv_row,
                "min",
                format!("{} min must be <= max", row.id),
            ));
        }
        if row.default < row.min || row.default > row.max {
            errors.push(ValidationError::at(
                "stats.csv",
                csv_row,
                "default",
                format!("{} default must be within min/max", row.id),
            ));
        }
        validate_locale_key(
            default_locale,
            "stats.csv",
            csv_row,
            "label_key",
            &row.label_key,
            &mut errors,
        );
    }

    for (index, row) in config.tables.resources.iter().enumerate() {
        let csv_row = csv_row(index);
        if row.default < 0.0 {
            errors.push(ValidationError::at(
                "resources.csv",
                csv_row,
                "default",
                format!("{} default must be >= 0", row.id),
            ));
        }
        validate_locale_key(
            default_locale,
            "resources.csv",
            csv_row,
            "label_key",
            &row.label_key,
            &mut errors,
        );
    }

    for (index, row) in config.tables.actions.iter().enumerate() {
        let csv_row = csv_row(index);
        if row.cost_energy < 0.0 || row.cost_wood < 0.0 || row.cost_fiber < 0.0 {
            errors.push(ValidationError::at(
                "actions.csv",
                csv_row,
                "cost_energy",
                format!("{} costs must be >= 0", row.id),
            ));
        }
        if !registered_action_resolvers().contains(row.resolver_id.as_str()) {
            errors.push(ValidationError::at(
                "actions.csv",
                csv_row,
                "resolver_id",
                format!("resolver_id {:?} is not registered", row.resolver_id),
            ));
        }
        validate_locale_key(
            default_locale,
            "actions.csv",
            csv_row,
            "name_key",
            &row.name_key,
            &mut errors,
        );
    }

    for (index, row) in config.tables.events.iter().enumerate() {
        let csv_row = csv_row(index);
        if row.base_weight < 0.0 {
            errors.push(ValidationError::at(
                "events.csv",
                csv_row,
                "base_weight",
                format!("{} base_weight must be >= 0", row.id),
            ));
        }
        if !registered_event_resolvers().contains(row.resolver_id.as_str()) {
            errors.push(ValidationError::at(
                "events.csv",
                csv_row,
                "resolver_id",
                format!("resolver_id {:?} is not registered", row.resolver_id),
            ));
        }
        validate_locale_key(
            default_locale,
            "events.csv",
            csv_row,
            "title_key",
            &row.title_key,
            &mut errors,
        );
    }

    for (index, row) in config.tables.panels.iter().enumerate() {
        validate_locale_key(
            default_locale,
            "panels.csv",
            csv_row(index),
            "title_key",
            &row.title_key,
            &mut errors,
        );
    }

    for (index, row) in config.tables.prompts.iter().enumerate() {
        validate_locale_key(
            default_locale,
            "prompts.csv",
            csv_row(index),
            "template_key",
            &row.template_key,
            &mut errors,
        );
    }

    errors
}

fn validate_ids<'a>(
    location: &str,
    ids: impl Iterator<Item = (usize, &'a str)>,
    errors: &mut Vec<ValidationError>,
) {
    let mut seen = HashSet::new();
    for (row, id) in ids {
        if id.trim().is_empty() {
            errors.push(ValidationError::at(location, row, "id", "id is required"));
        } else if !seen.insert(id.to_string()) {
            errors.push(ValidationError::at(
                location,
                row,
                "id",
                format!("duplicate id {:?}", id),
            ));
        }
    }
}

fn validate_locale_key(
    default_locale: Option<&std::collections::HashMap<String, String>>,
    location: &str,
    row: usize,
    column: &str,
    key: &str,
    errors: &mut Vec<ValidationError>,
) {
    if key.trim().is_empty() {
        errors.push(ValidationError::at(
            location,
            row,
            column,
            "locale key is required",
        ));
        return;
    }
    if let Some(locale) = default_locale
        && !locale.contains_key(key)
    {
        errors.push(ValidationError::at(
            location,
            row,
            column,
            format!("missing locale key {:?}", key),
        ));
    }
}

fn csv_row(index: usize) -> usize {
    index + 2
}

pub fn registered_action_resolvers() -> HashSet<&'static str> {
    [
        "fish_basic",
        "eat_food_basic",
        "collect_rain_basic",
        "salvage_basic",
        "repair_basic",
        "rest_basic",
        "observe_weather_basic",
        "study_chart_basic",
        "change_course_basic",
    ]
    .into_iter()
    .collect()
}

pub fn registered_event_resolvers() -> HashSet<&'static str> {
    [
        "rain_basic",
        "heat_basic",
        "storm_basic",
        "fish_shoal_basic",
        "floating_crate_basic",
        "hull_damage_basic",
        "island_silhouette_basic",
        "abandoned_ship_basic",
    ]
    .into_iter()
    .collect()
}
