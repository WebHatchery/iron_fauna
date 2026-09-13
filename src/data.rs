//! Embedded game data: definitions loaded from assets/data/*.json.

pub mod balance;
pub mod factory;
pub mod graftware;
pub mod item;
pub mod quest;
pub mod settlement;
pub mod species;
pub mod world;

mod validation;

use balance::BalanceConfig;
use factory::FactoryDef;
use graftware::GraftwareDef;
use item::ConsumableDef;
use macroquad_toolkit::assets::TextureConfig;
use macroquad_toolkit::data_loader::{load_embedded_json_labeled, DataRegistry};
use quest::QuestDef;
use serde::{Deserialize, Serialize};
use settlement::SettlementDef;
use species::SpeciesDef;
use world::{MapKind, WorldDef};

const GAME_CONFIG_JSON: &str =
    macroquad_toolkit::include_json_str!("../assets/data/game_config.json");
const BALANCE_JSON: &str = macroquad_toolkit::include_json_str!("../assets/data/balance.json");
const SPECIES_JSON: &str = macroquad_toolkit::include_json_str!("../assets/data/species.json");
const GRAFTWARE_JSON: &str = macroquad_toolkit::include_json_str!("../assets/data/graftware.json");
const WORLD_JSON: &str = macroquad_toolkit::include_json_str!("../assets/data/world.json");
const SETTLEMENTS_JSON: &str =
    macroquad_toolkit::include_json_str!("../assets/data/settlements.json");
const QUESTS_JSON: &str = macroquad_toolkit::include_json_str!("../assets/data/quests.json");
const ITEMS_JSON: &str = macroquad_toolkit::include_json_str!("../assets/data/items.json");
const FACTORIES_JSON: &str = macroquad_toolkit::include_json_str!("../assets/data/factories.json");
const TEXTURE_MANIFEST_JSON: &str =
    macroquad_toolkit::include_json_str!("../assets/data/texture_manifest.json");

const REGION_PACKS: [(&str, &str); 5] = [
    (
        "mirrormere",
        macroquad_toolkit::include_json_str!("../assets/data/regions/mirrormere.json"),
    ),
    (
        "stormcap",
        macroquad_toolkit::include_json_str!("../assets/data/regions/stormcap.json"),
    ),
    (
        "ashvein",
        macroquad_toolkit::include_json_str!("../assets/data/regions/ashvein.json"),
    ),
    (
        "sporefen",
        macroquad_toolkit::include_json_str!("../assets/data/regions/sporefen.json"),
    ),
    (
        "bonewhite",
        macroquad_toolkit::include_json_str!("../assets/data/regions/bonewhite.json"),
    ),
];

#[derive(Debug, Clone, Deserialize)]
struct RegionPack {
    #[serde(default)]
    maps: Vec<world::MapDef>,
    #[serde(default)]
    settlements: Vec<SettlementDef>,
    #[serde(default)]
    factories: Vec<FactoryDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub game_name: String,
    pub display_name: String,
    pub save_slot: String,
    pub version: String,
    pub starter: StarterConfig,
    pub controls: ControlsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlsConfig {
    pub overworld: String,
    pub dialogue: String,
    pub battle: String,
    pub battle_menu: String,
    pub codex: String,
    pub outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarterConfig {
    pub rider_name: String,
    pub starting_scrip: i64,
    pub species_id: String,
    pub initial_grafts: Vec<String>,
    pub equipment: Vec<StarterEquipment>,
    pub consumables: Vec<StarterConsumable>,
    pub journal_entry: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarterEquipment {
    pub limb_id: String,
    pub slot: usize,
    pub graft_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarterConsumable {
    pub item_id: String,
    pub count: u32,
}

#[derive(Debug, Clone)]
pub struct GameData {
    pub config: GameConfig,
    pub balance: BalanceConfig,
    pub species: DataRegistry<SpeciesDef>,
    pub graftware: DataRegistry<GraftwareDef>,
    pub world: WorldDef,
    pub settlements: DataRegistry<SettlementDef>,
    pub quests: DataRegistry<QuestDef>,
    pub items: DataRegistry<ConsumableDef>,
    pub factories: DataRegistry<FactoryDef>,
    pub texture_manifest: Vec<TextureConfig>,
}

impl GameData {
    pub fn load() -> Result<Self, String> {
        let config = load_embedded_json_labeled("game_config", GAME_CONFIG_JSON)?;
        let balance = load_embedded_json_labeled("balance", BALANCE_JSON)?;
        let species = DataRegistry::from_embedded_json(SPECIES_JSON, "id")
            .map_err(|error| format!("species: {error}"))?;
        let graftware = DataRegistry::from_embedded_json(GRAFTWARE_JSON, "id")
            .map_err(|error| format!("graftware: {error}"))?;
        let mut world: WorldDef = load_embedded_json_labeled("world", WORLD_JSON)?;
        let mut settlements: DataRegistry<SettlementDef> =
            DataRegistry::from_embedded_json(SETTLEMENTS_JSON, "id")
                .map_err(|error| format!("settlements: {error}"))?;
        let quests: DataRegistry<QuestDef> = DataRegistry::from_embedded_json(QUESTS_JSON, "id")
            .map_err(|error| format!("quests: {error}"))?;
        let items: DataRegistry<ConsumableDef> = DataRegistry::from_embedded_json(ITEMS_JSON, "id")
            .map_err(|error| format!("items: {error}"))?;
        let mut factories: DataRegistry<FactoryDef> =
            DataRegistry::from_embedded_json(FACTORIES_JSON, "id")
                .map_err(|error| format!("factories: {error}"))?;
        let texture_manifest =
            load_embedded_json_labeled("texture_manifest", TEXTURE_MANIFEST_JSON)?;

        for (name, json) in REGION_PACKS {
            let pack: RegionPack = load_embedded_json_labeled(name, json)?;
            world.maps.extend(pack.maps);
            for settlement in pack.settlements {
                if settlements.contains(&settlement.id) {
                    return Err(format!(
                        "content validation: duplicate settlement id '{}' while merging {}",
                        settlement.id, name
                    ));
                }
                settlements.insert(settlement.id.clone(), settlement);
            }
            for factory in pack.factories {
                if factories.contains(&factory.id) {
                    return Err(format!(
                        "content validation: duplicate factory id '{}' while merging {}",
                        factory.id, name
                    ));
                }
                factories.insert(factory.id.clone(), factory);
            }
        }

        for (_, factory) in factories.iter() {
            for floor in &factory.floors {
                let mut floor = floor.clone();
                floor.kind = MapKind::Factory;
                floor.factory_id = Some(factory.id.clone());
                world.maps.push(floor);
            }
        }

        let data = Self {
            config,
            balance,
            species,
            graftware,
            world,
            settlements,
            quests,
            items,
            factories,
            texture_manifest,
        };
        data.validate()?;
        Ok(data)
    }

    /// Validate cross-file content after registry and regional merges.
    pub fn validate(&self) -> Result<(), String> {
        validation::validate(self)
    }
}
