use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::hash::Hasher;
use std::path::{Path, PathBuf};

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
    let root = PathBuf::from("scenarios").join(scenario_id);
    load_scenario_config_from_root(root)
}

pub fn load_scenario_config_from_root(root: impl AsRef<Path>) -> Result<LoadedScenarioConfig> {
    let root = root.as_ref().to_path_buf();
    let manifest_path = root.join("scenario.toml");
    let manifest_text = fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    let manifest: Manifest = toml::from_str(&manifest_text)
        .with_context(|| format!("parsing {}", manifest_path.display()))?;

    let stats = read_csv::<StatRow>(&root.join(&manifest.tables.stats))?;
    let resources = read_csv::<ResourceRow>(&root.join(&manifest.tables.resources))?;
    let actions = read_csv::<ActionRow>(&root.join(&manifest.tables.actions))?;
    let events = read_csv::<EventRow>(&root.join(&manifest.tables.events))?;
    let balance_rows = read_csv::<BalanceRow>(&root.join(&manifest.tables.balance))?;
    let panels = read_csv::<PanelRow>(&root.join(&manifest.tables.panels))?;
    let prompts = match &manifest.tables.prompts {
        Some(path) => read_csv::<PromptRow>(&root.join(path))?,
        None => Vec::new(),
    };

    if let Some(path) = &manifest.tables.event_weights {
        let _ = fs::read_to_string(root.join(path))
            .with_context(|| format!("reading optional event weights table {}", path))?;
    }

    let mut balance = HashMap::new();
    for row in balance_rows {
        balance.insert(row.key, row.value);
    }

    let mut locales = HashMap::new();
    for (locale_key, path) in &manifest.locales {
        let locale = normalize_locale_key(locale_key);
        let table = read_locale_csv(&root.join(path), &locale)?;
        locales.insert(locale, table);
    }

    let config_hash = stable_config_hash(&root, &manifest)?;

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

fn read_csv<T>(path: &Path) -> Result<Vec<T>>
where
    T: for<'de> Deserialize<'de>,
{
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(path)
        .with_context(|| format!("opening {}", path.display()))?;
    let mut rows = Vec::new();
    for result in reader.deserialize() {
        rows.push(result.with_context(|| format!("parsing {}", path.display()))?);
    }
    Ok(rows)
}

fn read_locale_csv(path: &Path, locale: &str) -> Result<LocaleTable> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(path)
        .with_context(|| format!("opening {}", path.display()))?;
    let headers = reader.headers()?.clone();
    if !headers.iter().any(|h| h == "key") || !headers.iter().any(|h| h == "text") {
        return Err(anyhow!("{} must contain key,text columns", path.display()));
    }

    let mut entries = HashMap::new();
    for record in reader.records() {
        let record = record?;
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

fn stable_config_hash(root: &Path, manifest: &Manifest) -> Result<String> {
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
        let contents = fs::read(root.join(&file))
            .with_context(|| format!("reading {} for config hash", root.join(&file).display()))?;
        hasher.write(&contents);
    }
    Ok(format!("{:016x}", hasher.finish()))
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
