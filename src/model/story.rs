//! Story progression: flag-gated dialogue selection and one-shot rewards.

use crate::data::world::{DialogueCond, DialogueRule, NpcDef};
use crate::data::GameData;
use crate::model::inventory::GraftCondition;
use crate::model::worldstate::Verdict;
use crate::state::GameSession;

fn verdict_matches(name: &str, verdict: Option<Verdict>) -> bool {
    matches!(
        (name, verdict),
        ("Purge", Some(Verdict::Purge))
            | ("Reseed", Some(Verdict::Reseed))
            | ("Bind", Some(Verdict::Bind))
    )
}

pub fn cond_passes(cond: &DialogueCond, session: &GameSession) -> bool {
    if !cond
        .flags_all
        .iter()
        .all(|f| session.story_flags.contains(f))
    {
        return false;
    }
    if cond
        .flags_none
        .iter()
        .any(|f| session.story_flags.contains(f))
    {
        return false;
    }
    if let Some(factory) = &cond.heart_defeated {
        if !session.world_state.factory(factory).heart_defeated {
            return false;
        }
    }
    if let Some((factory, verdict)) = &cond.verdict {
        if !verdict_matches(verdict, session.world_state.verdict(factory)) {
            return false;
        }
    }
    if let Some((factory, wanted)) = &cond.relapsed {
        if session.world_state.factory(factory).relapsed != *wanted {
            return false;
        }
    }
    if let Some(quest) = &cond.quest_active {
        if !session.quests.is_active(quest) {
            return false;
        }
    }
    if let Some(quest) = &cond.quest_ready {
        if !session.quests.is_ready(quest) {
            return false;
        }
    }
    if let Some(quest) = &cond.quest_done {
        if !session.quests.is_done(quest) {
            return false;
        }
    }
    if let Some(quest) = &cond.quest_none {
        if !session.quests.is_untaken(quest) {
            return false;
        }
    }
    true
}

/// Picks the first dialogue rule whose condition passes, falling back to the
/// NPC's plain `lines`.
pub fn select_dialogue<'a>(
    npc: &'a NpcDef,
    session: &GameSession,
) -> Option<DialogueSelection<'a>> {
    for rule in &npc.dialogue {
        let passes = rule
            .when
            .as_ref()
            .map(|c| cond_passes(c, session))
            .unwrap_or(true);
        if passes {
            return Some(DialogueSelection {
                lines: &rule.lines,
                rule: Some(rule),
            });
        }
    }
    if npc.lines.is_empty() {
        None
    } else {
        Some(DialogueSelection {
            lines: &npc.lines,
            rule: None,
        })
    }
}

pub struct DialogueSelection<'a> {
    pub lines: &'a [String],
    pub rule: Option<&'a DialogueRule>,
}

/// Applies a rule's effects (flags, rewards). Rewards are inherently
/// one-shot: gate them behind a flag in `flags_none` + `set_flags`.
/// Returns notification lines.
pub fn apply_dialogue_effects(
    rule: &DialogueRule,
    session: &mut GameSession,
    data: &GameData,
) -> Vec<String> {
    let mut notes = Vec::new();
    for flag in &rule.set_flags {
        session.story_flags.insert(flag.clone());
    }
    if rule.give_scrip > 0 {
        session.profile.inventory.scrip += rule.give_scrip;
        notes.push(format!("Received {} scrip", rule.give_scrip));
    }
    for def_id in &rule.give_grafts {
        session.profile.grant_graft(def_id, GraftCondition::Intact);
        if let Some(def) = data.graftware.get(def_id) {
            notes.push(format!("Received: {}", def.name));
        }
    }
    if let Some(quest_id) = &rule.start_quest {
        if let Some(note) = crate::model::quest::start(session, data, quest_id) {
            notes.push(note);
        }
    }
    if let Some(quest_id) = &rule.complete_quest {
        notes.extend(crate::model::quest::complete(session, data, quest_id));
    }
    notes
}

#[cfg(test)]
mod tests;
