//! Session state, settings, and save data.

use crate::data::GameData;
use crate::model::creature::CreatureOrigin;
use crate::model::inventory::GraftCondition;
use crate::model::Profile;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Battle pacing (`combat.md` §1): Wait auto-pauses at decision points;
/// Active never does. A tempo lever, not two combat modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaceSetting {
    Wait,
    Active,
}

impl PaceSetting {
    pub fn display_name(self) -> &'static str {
        match self {
            PaceSetting::Wait => "Wait",
            PaceSetting::Active => "Active",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            PaceSetting::Wait => PaceSetting::Active,
            PaceSetting::Active => PaceSetting::Wait,
        }
    }
}

/// Where the player stands in the overworld.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    pub map_id: String,
    pub x: i32,
    pub y: i32,
}

/// The live game session: everything that persists into a save.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSession {
    pub profile: Profile,
    pub pace: PaceSetting,
    /// Total battles fought (drives bond growth and pacing stats).
    pub battles_fought: u32,
    pub location: Location,
    /// Overworld steps taken — seeds encounter randomness deterministically.
    pub steps: u64,
    /// The sum of the player's factory verdicts (`game_design.md` §9.2).
    #[serde(default)]
    pub world_state: crate::model::worldstate::WorldState,
    /// Story progression flags set by dialogue and events.
    #[serde(default)]
    pub story_flags: std::collections::HashSet<String>,
    /// Accepted / in-progress / finished quests.
    #[serde(default)]
    pub quests: crate::model::quest::QuestLog,
    /// Auto-generated chronicle of the rider's deeds (codex Journal tab).
    #[serde(default)]
    pub journal: Vec<crate::model::journal::JournalEntry>,
}

impl GameSession {
    /// A fresh game: the orphan rider, a starter core, and a few salvaged
    /// parts to graft.
    pub fn new_game(data: &GameData) -> Self {
        let starter_data = &data.config.starter;
        let mut profile = Profile::new(&starter_data.rider_name, starter_data.starting_scrip);
        let starter =
            profile.spawn_creature(data, &starter_data.species_id, CreatureOrigin::Starter);
        let mut starter_items = std::collections::HashMap::new();
        for graft in &starter_data.initial_grafts {
            let item_id = profile.grant_graft(graft, GraftCondition::Intact);
            starter_items.insert(graft.as_str(), item_id);
        }
        for consumable in &starter_data.consumables {
            profile
                .inventory
                .add_consumable(&consumable.item_id, consumable.count);
        }

        let inventory = profile.inventory.clone();
        if let Some(creature) = profile.roster.creature_mut(starter) {
            for equipment in &starter_data.equipment {
                let Some(&item_id) = starter_items.get(equipment.graft_id.as_str()) else {
                    eprintln!(
                        "starter setup: equipment '{}' has no granted inventory item",
                        equipment.graft_id
                    );
                    continue;
                };
                if let Err(error) = creature.equip(
                    data,
                    &inventory,
                    &equipment.limb_id,
                    equipment.slot,
                    item_id,
                ) {
                    eprintln!(
                        "starter setup: could not equip '{}' on '{}'/{}: {}",
                        equipment.graft_id,
                        equipment.limb_id,
                        equipment.slot,
                        error.message()
                    );
                }
            }
        }

        let start = data
            .world
            .map(&data.world.start_map)
            .expect("world start map missing");

        Self {
            profile,
            pace: PaceSetting::Wait,
            battles_fought: 0,
            location: Location {
                map_id: start.id.clone(),
                x: start.spawn_x,
                y: start.spawn_y,
            },
            steps: 0,
            world_state: Default::default(),
            story_flags: Default::default(),
            quests: Default::default(),
            journal: vec![crate::model::journal::JournalEntry {
                step: 0,
                text: starter_data.journal_entry.clone(),
            }],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: String,
    pub session: GameSession,
}

impl SaveData {
    pub fn from_session(session: &GameSession, version: &str) -> Self {
        Self {
            version: version.to_owned(),
            session: session.clone(),
        }
    }
}

/// Save migration hook. Pre-release there are no legacy formats — anything
/// that doesn't parse as current is rejected.
pub fn migrate_save_value(
    detected_version: Option<String>,
    value: Value,
    config_version: &str,
) -> Result<SaveData, String> {
    let payload = value.get("data").cloned().unwrap_or(value);
    match serde_json::from_value::<SaveData>(payload) {
        Ok(mut save) => {
            save.version = config_version.to_owned();
            Ok(save)
        }
        Err(err) => Err(format!(
            "Unsupported save format {:?}: {}",
            detected_version, err
        )),
    }
}
