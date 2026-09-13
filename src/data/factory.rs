//! Gestarium (bio-factory) definitions — the game's dungeons and the anchor
//! of the verdict system (`game_design.md` §7, §9).

use crate::data::settlement::DuelUnitDef;
use crate::data::world::MapDef;
use crate::model::rider::RiderUpgrade;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryDef {
    pub id: String,
    pub name: String,
    pub region: String,
    pub description: String,
    /// Floor maps, entrance first, heart floor last. Merged into the world
    /// map list at load time so traversal works unchanged.
    pub floors: Vec<MapDef>,
    /// The authored heart-guardian fight on the deepest floor.
    pub heart_guard: Vec<DuelUnitDef>,
    /// The rider's permanent upgrade for silencing this factory (§3).
    pub rider_upgrade: RiderUpgrade,
    /// Species this factory can gestate for you once Bound (§9 Bind).
    #[serde(default)]
    pub grows: Vec<String>,
    /// Scrip cost to grow a core here once Bound.
    #[serde(default)]
    pub grow_cost: i64,
}
