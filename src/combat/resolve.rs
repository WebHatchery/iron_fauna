//! Applies a finished battle back to the player's profile: captures, salvage,
//! scrip, graft damage, and bond growth.

use crate::combat::engine::Battle;
use crate::combat::{BattleContext, BattleOutcome, Side};
use crate::data::GameData;
use crate::model::creature::CreatureOrigin;
use crate::model::inventory::GraftCondition;
use crate::state::GameSession;

#[derive(Debug, Clone, Default)]
pub struct ResolutionSummary {
    pub lines: Vec<String>,
}

/// Call exactly once, after `battle.outcome` is set.
pub fn apply(session: &mut GameSession, data: &GameData, battle: &Battle) -> ResolutionSummary {
    let mut summary = ResolutionSummary::default();
    let Some(outcome) = &battle.outcome else {
        return summary;
    };
    session.battles_fought += 1;
    // Consumables spent in the fight (potions, loaded ammo) are gone for good.
    session.profile.inventory.consumables = battle.bag.clone();

    // Player graftware that was destroyed or detached in the fight comes home
    // Damaged — repairable at a settlement, never free (`game_design.md` §4.4).
    for unit in battle.units.iter().filter(|u| u.side == Side::Player) {
        for mount in &unit.mounts {
            if let Some(item_id) = mount.item_id.filter(|_| mount.destroyed || mount.detached) {
                session.profile.inventory.mark_damaged(item_id);
                if let Some(item) = session.profile.inventory.item(item_id) {
                    if let Some(def) = data.graftware.get(&item.def_id) {
                        summary.lines.push(format!("{} needs repair", def.name));
                    }
                }
            }
        }
        // Fighting together deepens the bond, win or lose.
        if let Some(creature_id) = unit.creature_id {
            if let Some(c) = session.profile.roster.creature_mut(creature_id) {
                c.add_bond(0.15);
            }
        }
    }
    // Loadouts drop references to now-damaged parts.
    let inventory = session.profile.inventory.clone();
    for c in &mut session.profile.roster.creatures {
        c.prune_loadout(&inventory);
    }

    match outcome {
        BattleOutcome::Victory(rewards) => {
            for species_id in &rewards.captured_species {
                let origin = match battle.context {
                    BattleContext::FactoryDismantle => CreatureOrigin::Freed {
                        factory_id: "unknown".to_owned(),
                    },
                    _ => CreatureOrigin::Wild,
                };
                session.profile.spawn_creature(data, species_id, origin);
                let name = data
                    .species
                    .get(species_id)
                    .map(|s| s.name.as_str())
                    .unwrap_or(species_id);
                summary.lines.push(match battle.context {
                    BattleContext::FactoryDismantle => format!("{} freed from its war-body", name),
                    _ => format!("{} subdued and befriended", name),
                });
            }
            for def_id in &rewards.salvage {
                session.profile.grant_graft(def_id, GraftCondition::Intact);
                if let Some(def) = data.graftware.get(def_id) {
                    summary.lines.push(format!("Salvaged: {}", def.name));
                }
            }
            if rewards.scrip > 0 {
                session.profile.inventory.scrip += rewards.scrip;
                summary
                    .lines
                    .push(format!("Earned {} scrip", rewards.scrip));
            }
        }
        BattleOutcome::Fled => {
            summary
                .lines
                .push("Every core cracked — you fled and escaped.".to_owned());
        }
    }
    summary
}
