use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::hash::Hasher;
use std::path::{Path, PathBuf};

type EmbeddedScenarioFiles = &'static [(&'static str, &'static str)];

const SEA_SURVIVAL_FILES: EmbeddedScenarioFiles = &[
    (
        "scenario.toml",
        include_str!("../../scenarios/sea_survival/scenario.toml"),
    ),
    (
        "locales/en-US.csv",
        include_str!("../../scenarios/sea_survival/locales/en-US.csv"),
    ),
    (
        "locales/zh-CN.csv",
        include_str!("../../scenarios/sea_survival/locales/zh-CN.csv"),
    ),
    (
        "tables/actions.csv",
        include_str!("../../scenarios/sea_survival/tables/actions.csv"),
    ),
    (
        "tables/balance.csv",
        include_str!("../../scenarios/sea_survival/tables/balance.csv"),
    ),
    (
        "tables/event_weights.csv",
        include_str!("../../scenarios/sea_survival/tables/event_weights.csv"),
    ),
    (
        "tables/events.csv",
        include_str!("../../scenarios/sea_survival/tables/events.csv"),
    ),
    (
        "tables/panels.csv",
        include_str!("../../scenarios/sea_survival/tables/panels.csv"),
    ),
    (
        "tables/prompts.csv",
        include_str!("../../scenarios/sea_survival/tables/prompts.csv"),
    ),
    (
        "tables/resources.csv",
        include_str!("../../scenarios/sea_survival/tables/resources.csv"),
    ),
    (
        "tables/stats.csv",
        include_str!("../../scenarios/sea_survival/tables/stats.csv"),
    ),
];

const DESERT_OUTPOST_FILES: EmbeddedScenarioFiles = &[
    (
        "scenario.toml",
        include_str!("../../scenarios/desert_outpost/scenario.toml"),
    ),
    (
        "locales/en-US.csv",
        include_str!("../../scenarios/desert_outpost/locales/en-US.csv"),
    ),
    (
        "locales/zh-CN.csv",
        include_str!("../../scenarios/desert_outpost/locales/zh-CN.csv"),
    ),
    (
        "tables/actions.csv",
        include_str!("../../scenarios/desert_outpost/tables/actions.csv"),
    ),
    (
        "tables/balance.csv",
        include_str!("../../scenarios/desert_outpost/tables/balance.csv"),
    ),
    (
        "tables/event_weights.csv",
        include_str!("../../scenarios/desert_outpost/tables/event_weights.csv"),
    ),
    (
        "tables/events.csv",
        include_str!("../../scenarios/desert_outpost/tables/events.csv"),
    ),
    (
        "tables/panels.csv",
        include_str!("../../scenarios/desert_outpost/tables/panels.csv"),
    ),
    (
        "tables/prompts.csv",
        include_str!("../../scenarios/desert_outpost/tables/prompts.csv"),
    ),
    (
        "tables/resources.csv",
        include_str!("../../scenarios/desert_outpost/tables/resources.csv"),
    ),
    (
        "tables/stats.csv",
        include_str!("../../scenarios/desert_outpost/tables/stats.csv"),
    ),
];

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub version: String,
    pub default_locale: String,
    pub entry: String,
    pub tables: TablePaths,
    pub locales: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TablePaths {
    pub stats: String,
    pub resources: String,
    pub actions: String,
    pub events: String,
    pub event_weights: Option<String>,
    pub balance: String,
    pub panels: String,
    pub prompts: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StatRow {
    pub id: String,
    pub label_key: String,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    pub display_format: String,
    pub sort: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResourceRow {
    pub id: String,
    pub label_key: String,
    pub default: f64,
    pub unit: String,
    pub display_format: String,
    pub sort: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActionRow {
    pub id: String,
    pub name_key: String,
    pub risk: String,
    pub cost_energy: f64,
    pub cost_wood: f64,
    pub cost_fiber: f64,
    pub reward_type: String,
    pub resolver_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EventRow {
    pub id: String,
    pub title_key: String,
    pub severity: String,
    pub base_weight: f64,
    pub cooldown_ticks: u64,
    pub resolver_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BalanceRow {
    pub key: String,
    pub value: f64,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PanelRow {
    pub id: String,
    pub title_key: String,
    pub level: u8,
    pub sort: i32,
    pub visible: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PromptRow {
    pub id: String,
    pub template_key: String,
    pub variables: String,
}

#[derive(Debug, Clone)]
pub struct LocaleTable {
    pub locale: String,
    pub entries: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ScenarioTables {
    pub stats: Vec<StatRow>,
    pub resources: Vec<ResourceRow>,
    pub actions: Vec<ActionRow>,
    pub events: Vec<EventRow>,
    pub balance: HashMap<String, f64>,
    pub panels: Vec<PanelRow>,
    pub prompts: Vec<PromptRow>,
}

#[derive(Debug, Clone)]
pub struct LoadedScenarioConfig {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub tables: ScenarioTables,
    pub locales: HashMap<String, LocaleTable>,
    pub config_hash: String,
}

pub fn load_scenario_config(scenario_id: &str) -> Result<LoadedScenarioConfig> {
    load_scenario_config_with_scenarios_dir(scenario_id, None)
}

pub fn load_scenario_config_with_scenarios_dir(
    scenario_id: &str,
    scenarios_dir: Option<&Path>,
) -> Result<LoadedScenarioConfig> {
    let root = scenarios_dir
        .unwrap_or_else(|| Path::new("scenarios"))
        .join(scenario_id);
    if scenarios_dir.is_some() || root.exists() {
        return load_scenario_config_from_root(root);
    }

    load_embedded_scenario_config(scenario_id)
        .with_context(|| format!("scenario {scenario_id} was not found in {}", root.display()))
}

pub fn load_scenario_config_from_root(root: impl AsRef<Path>) -> Result<LoadedScenarioConfig> {
    let root = root.as_ref().to_path_buf();
    let manifest_path = root.join("scenario.toml");
    let manifest_text = fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    load_scenario_config_from_texts(
        root.clone(),
        manifest_path.display().to_string(),
        manifest_text,
        |path| {
            let file_path = root.join(path);
            fs::read_to_string(&file_path)
                .with_context(|| format!("reading {}", file_path.display()))
        },
    )
}

fn load_embedded_scenario_config(scenario_id: &str) -> Result<LoadedScenarioConfig> {
    let files = embedded_files_for(scenario_id)
        .ok_or_else(|| anyhow!("scenario {scenario_id} is not embedded"))?;
    let manifest_text = embedded_file_text(files, "scenario.toml")?.to_string();
    load_scenario_config_from_texts(
        PathBuf::from(format!("<embedded>/{scenario_id}")),
        format!("embedded scenario {scenario_id}/scenario.toml"),
        manifest_text,
        |path| embedded_file_text(files, path).map(str::to_string),
    )
}

fn load_scenario_config_from_texts(
    root: PathBuf,
    manifest_label: String,
    manifest_text: String,
    mut read_text: impl FnMut(&str) -> Result<String>,
) -> Result<LoadedScenarioConfig> {
    let manifest: Manifest =
        toml::from_str(&manifest_text).with_context(|| format!("parsing {manifest_label}"))?;

    let stats =
        read_csv_text::<StatRow>(&manifest.tables.stats, &read_text(&manifest.tables.stats)?)?;
    let resources = read_csv_text::<ResourceRow>(
        &manifest.tables.resources,
        &read_text(&manifest.tables.resources)?,
    )?;
    let actions = read_csv_text::<ActionRow>(
        &manifest.tables.actions,
        &read_text(&manifest.tables.actions)?,
    )?;
    let events = read_csv_text::<EventRow>(
        &manifest.tables.events,
        &read_text(&manifest.tables.events)?,
    )?;
    let balance_rows = read_csv_text::<BalanceRow>(
        &manifest.tables.balance,
        &read_text(&manifest.tables.balance)?,
    )?;
    let panels = read_csv_text::<PanelRow>(
        &manifest.tables.panels,
        &read_text(&manifest.tables.panels)?,
    )?;
    let prompts = match &manifest.tables.prompts {
        Some(path) => read_csv_text::<PromptRow>(path, &read_text(path)?)?,
        None => Vec::new(),
    };

    if let Some(path) = &manifest.tables.event_weights {
        let _ = read_text(path)
            .with_context(|| format!("reading optional event weights table {}", path))?;
    }

    let mut balance = HashMap::new();
    for row in balance_rows {
        balance.insert(row.key, row.value);
    }

    let mut locales = HashMap::new();
    for (locale_key, path) in &manifest.locales {
        let locale = normalize_locale_key(locale_key);
        let table = read_locale_csv_text(path, &locale, &read_text(path)?)?;
        locales.insert(locale, table);
    }

    let config_hash =
        stable_config_hash_from_texts(&manifest, &manifest_text, |path| read_text(path))?;

    Ok(LoadedScenarioConfig {
        root,
        manifest,
        tables: ScenarioTables {
            stats,
            resources,
            actions,
            events,
            balance,
            panels,
            prompts,
        },
        locales,
        config_hash,
    })
}

fn read_csv_text<T>(label: &str, text: &str) -> Result<Vec<T>>
where
    T: for<'de> Deserialize<'de>,
{
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());
    let mut rows = Vec::new();
    for result in reader.deserialize() {
        rows.push(result.with_context(|| format!("parsing {label}"))?);
    }
    Ok(rows)
}

fn read_locale_csv_text(label: &str, locale: &str, text: &str) -> Result<LocaleTable> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());
    let headers = reader
        .headers()
        .with_context(|| format!("reading headers from {label}"))?
        .clone();
    if !headers.iter().any(|h| h == "key") || !headers.iter().any(|h| h == "text") {
        return Err(anyhow!("{label} must contain key,text columns"));
    }

    let mut entries = HashMap::new();
    for record in reader.records() {
        let record = record.with_context(|| format!("parsing {label}"))?;
        let key = record
            .get(headers.iter().position(|h| h == "key").unwrap())
            .unwrap_or_default()
            .trim()
            .to_string();
        let text = record
            .get(headers.iter().position(|h| h == "text").unwrap())
            .unwrap_or_default()
            .trim()
            .to_string();
        if !key.is_empty() {
            entries.insert(key, text);
        }
    }

    Ok(LocaleTable {
        locale: locale.to_string(),
        entries,
    })
}

fn normalize_locale_key(key: &str) -> String {
    key.replace('_', "-")
}

fn stable_config_hash_from_texts(
    manifest: &Manifest,
    manifest_text: &str,
    mut read_text: impl FnMut(&str) -> Result<String>,
) -> Result<String> {
    let mut files = vec![
        "scenario.toml".to_string(),
        manifest.tables.stats.clone(),
        manifest.tables.resources.clone(),
        manifest.tables.actions.clone(),
        manifest.tables.events.clone(),
        manifest.tables.balance.clone(),
        manifest.tables.panels.clone(),
    ];
    if let Some(path) = &manifest.tables.event_weights {
        files.push(path.clone());
    }
    if let Some(path) = &manifest.tables.prompts {
        files.push(path.clone());
    }
    for path in manifest.locales.values() {
        files.push(path.clone());
    }
    files.sort();

    let mut hasher = Fnv64::default();
    for file in files {
        hasher.write(file.as_bytes());
        if file == "scenario.toml" {
            hasher.write(manifest_text.as_bytes());
        } else {
            let contents =
                read_text(&file).with_context(|| format!("reading {file} for config hash"))?;
            hasher.write(contents.as_bytes());
        }
    }
    Ok(format!("{:016x}", hasher.finish()))
}

fn embedded_files_for(scenario_id: &str) -> Option<EmbeddedScenarioFiles> {
    match scenario_id {
        "sea_survival" => Some(SEA_SURVIVAL_FILES),
        "desert_outpost" => Some(DESERT_OUTPOST_FILES),
        _ => None,
    }
}

fn embedded_file_text(files: EmbeddedScenarioFiles, path: &str) -> Result<&'static str> {
    files
        .iter()
        .find_map(|(file, text)| (*file == path).then_some(*text))
        .ok_or_else(|| anyhow!("embedded scenario file {path} not found"))
}

#[derive(Default)]
struct Fnv64(u64);

impl Hasher for Fnv64 {
    fn write(&mut self, bytes: &[u8]) {
        if self.0 == 0 {
            self.0 = 0xcbf29ce484222325;
        }
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_scenario_loads_without_filesystem_root() {
        let config =
            load_embedded_scenario_config("sea_survival").expect("embedded scenario should load");

        assert_eq!(config.root, PathBuf::from("<embedded>/sea_survival"));
        assert_eq!(config.manifest.id, "sea_survival");
        assert!(config.locales.contains_key("en-US"));
        assert!(config.locales.contains_key("zh-CN"));
        assert!(!config.config_hash.is_empty());
    }

    #[test]
    fn explicit_scenarios_dir_does_not_fallback_to_embedded() {
        let scenarios_dir = tempfile::tempdir().expect("tempdir should be created");
        let error =
            load_scenario_config_with_scenarios_dir("sea_survival", Some(scenarios_dir.path()))
                .expect_err("explicit directory should be authoritative");

        assert!(format!("{error:#}").contains("scenario.toml"));
    }
}
