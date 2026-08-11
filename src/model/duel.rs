//! Duel bookkeeping: antes, stake transfer, and ladder rank
//! (`game_design.md` §10). The battle itself runs on the normal engine with
//! `BattleContext::Duel`; this module handles what changes hands afterward.

use crate::data::settlement::DuelistDef;
use crate::data::GameData;
use crate::state::GameSession;

/// A duel accepted and about to be (or being) fought.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingDuel {
    pub settlement_id: String,
    pub duelist_id: String,
    /// Inventory item the player put up (staked lane only).
    pub my_stake: Option<u64>,
}

/// Items the player may legally stake: unequipped, intact, and valuable
/// enough. The equipped-loadout exclusion is the protected-loadout floor —
/// you cannot wager the graftware currently keeping your creatures standing.
pub fn eligible_stakes(session: &GameSession, data: &GameData, min_value: i64) -> Vec<u64> {
    let equipped = session.profile.equipped_item_ids();
    session
        .profile
        .inventory
        .items
        .iter()
        .filter(|item| item.is_usable() && !equipped.contains(&item.id))
        .filter(|item| {
            data.graftware
                .get(&item.def_id)
                .is_some_and(|def| def.value >= min_value)
        })
        .map(|item| item.id)
        .collect()
}

pub fn current_rank(session: &GameSession, settlement_id: &str) -> u32 {
    session
        .profile
        .rider
        .duel_ranks
        .get(settlement_id)
        .copied()
        .unwrap_or(0)
}

pub fn can_challenge(session: &GameSession, settlement_id: &str, duelist: &DuelistDef) -> bool {
    current_rank(session, settlement_id) >= duelist.rank_req
}

/// Applies a finished duel to the profile. Returns human-readable lines.
pub fn apply_duel_result(
    session: &mut GameSession,
    data: &GameData,
    pending: &PendingDuel,
    duelist: &DuelistDef,
    won: bool,
) -> Vec<String> {
    let mut lines = Vec::new();
    if won {
        session.profile.inventory.scrip += duelist.reward_scrip;
        lines.push(format!(
            "{} yields — {} scrip",
            duelist.name, duelist.reward_scrip
        ));
        if let Some(stake_def) = &duelist.stake {
            session
                .profile
                .grant_graft(stake_def, crate::model::inventory::GraftCondition::Intact);
            let name = data
                .graftware
                .get(stake_def)
                .map(|d| d.name.as_str())
                .unwrap_or(stake_def);
            lines.push(format!("Won the stake: {}", name));
        }
        // Beating a duelist at your rank advances the ladder.
        let rank = current_rank(session, &pending.settlement_id);
        if duelist.rank_req >= rank {
            session
                .profile
                .rider
                .duel_ranks
                .insert(pending.settlement_id.clone(), duelist.rank_req + 1);
            lines.push(format!("Ring rank is now {}", duelist.rank_req + 1));
        }
    } else if let Some(item_id) = pending.my_stake {
        // The loser hands over the staked part. Their creatures are unharmed.
        if let Some(item) = session.profile.inventory.item(item_id) {
            let name = data
                .graftware
                .get(&item.def_id)
                .map(|d| d.name.as_str())
                .unwrap_or("part");
            lines.push(format!("Lost the stake: {}", name));
        }
        session.profile.inventory.items.retain(|i| i.id != item_id);
    } else {
        lines.push("A practice loss costs nothing but pride.".to_owned());
    }
    lines
}

#[cfg(test)]
mod tests;
