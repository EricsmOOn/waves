pub mod desert_outpost;
pub mod sea_survival;

use crate::config::LoadedScenarioConfig;
use crate::core::{ActionOption, DomainEvent, Resolution, WorldEvent, WorldState};
use crate::i18n::Catalog;
use anyhow::{Result, bail};
use rand::rngs::StdRng;

pub trait Scenario {
    fn id(&self) -> &str;
    fn initial_state(&self) -> WorldState;
    fn apply_tick(&mut self, state: &mut WorldState) -> Vec<DomainEvent>;
    fn select_event(&mut self, state: &WorldState, rng: &mut StdRng) -> WorldEvent;
    fn available_actions(&self, state: &WorldState, event: &WorldEvent) -> Vec<ActionOption>;
    fn resolve_action(
        &self,
        state: &mut WorldState,
        event: &WorldEvent,
        action_id: &str,
        catalog: &Catalog,
        rng: &mut StdRng,
    ) -> Resolution;
    fn outcome(&self, state: &WorldState) -> Option<String>;
}

pub fn build_scenario(config: LoadedScenarioConfig) -> Result<Box<dyn Scenario>> {
    match config.manifest.entry.as_str() {
        "sea_survival" => Ok(Box::new(sea_survival::SeaSurvivalScenario::new(config))),
        "desert_outpost" => Ok(Box::new(desert_outpost::DesertOutpostScenario::new(config))),
        entry => bail!("unsupported scenario entry {entry:?}"),
    }
}
