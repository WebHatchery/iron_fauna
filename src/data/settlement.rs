//! Settlement definitions: shops and duelling rings
//! (`game_design.md` §10 — practice and staked NPC duels).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopEntry {
    pub graft: String,
    /// Price override; defaults to the graft's base value.
    #[serde(default)]
    pub price: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuelGraftFit {
    pub limb: String,
    pub slot: usize,
    pub graft: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuelUnitDef {
    pub species: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub grafts: Vec<DuelGraftFit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuelistDef {
    pub id: String,
    pub name: String,
    pub blurb: String,
    /// Ladder rank required to challenge (0 = open to anyone).
    #[serde(default)]
    pub rank_req: u32,
    /// Practice duels risk nothing; staked duels wager parts.
    pub practice: bool,
    pub reward_scrip: i64,
    /// Staked lane: the part this duelist puts up.
    #[serde(default)]
    pub stake: Option<String>,
    /// Staked lane: minimum base value of the part you must put up.
    #[serde(default)]
    pub min_stake_value: i64,
    pub party: Vec<DuelUnitDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementDef {
    pub id: String,
    pub name: String,
    pub region: String,
    pub description: String,
    #[serde(default)]
    pub shop: Vec<ShopEntry>,
    #[serde(default)]
    pub duelists: Vec<DuelistDef>,
}

impl SettlementDef {
    pub fn duelist(&self, id: &str) -> Option<&DuelistDef> {
        self.duelists.iter().find(|d| d.id == id)
    }
}
