use super::*;
use crate::data::world::{DialogueCond, DialogueRule, NpcDef};
use crate::model::worldstate::Verdict;

fn npc_with_rules(rules: Vec<DialogueRule>) -> NpcDef {
    NpcDef {
        id: "test".into(),
        x: 0,
        y: 0,
        name: "Test".into(),
        lines: vec!["fallback".into()],
        dialogue: rules,
    }
}

fn rule(when: Option<DialogueCond>, line: &str) -> DialogueRule {
    DialogueRule {
        when,
        lines: vec![line.to_owned()],
        set_flags: vec![],
        give_scrip: 0,
        give_grafts: vec![],
        start_quest: None,
        complete_quest: None,
    }
}

#[test]
fn dialogue_reacts_to_flags_and_verdicts() {
    let data = crate::data::GameData::load().unwrap();
    let mut session = GameSession::new_game(&data);
    let npc = npc_with_rules(vec![
        rule(
            Some(DialogueCond {
                verdict: Some(("the_cradle".into(), "Purge".into())),
                ..Default::default()
            }),
            "after purge",
        ),
        rule(
            Some(DialogueCond {
                flags_all: vec!["met".into()],
                ..Default::default()
            }),
            "again",
        ),
    ]);

    // No flags, no verdict: falls through to plain lines.
    assert_eq!(
        select_dialogue(&npc, &session).unwrap().lines[0],
        "fallback"
    );

    session.story_flags.insert("met".into());
    assert_eq!(select_dialogue(&npc, &session).unwrap().lines[0], "again");

    // A verdict outranks the flag rule (listed first).
    session.world_state.factory_mut("the_cradle").verdict = Some(Verdict::Purge);
    assert_eq!(
        select_dialogue(&npc, &session).unwrap().lines[0],
        "after purge"
    );
}

#[test]
fn effects_grant_once_when_gated_by_flags() {
    let data = crate::data::GameData::load().unwrap();
    let mut session = GameSession::new_game(&data);
    let reward = DialogueRule {
        when: Some(DialogueCond {
            flags_none: vec!["paid".into()],
            ..Default::default()
        }),
        lines: vec!["here, take this".into()],
        set_flags: vec!["paid".into()],
        give_scrip: 100,
        give_grafts: vec!["spark_coil".into()],
        start_quest: None,
        complete_quest: None,
    };
    let npc = npc_with_rules(vec![reward]);

    let scrip_before = session.profile.inventory.scrip;
    let items_before = session.profile.inventory.items.len();
    let selection = select_dialogue(&npc, &session).unwrap();
    let rule = selection.rule.unwrap().clone();
    let notes = apply_dialogue_effects(&rule, &mut session, &data);
    assert_eq!(session.profile.inventory.scrip, scrip_before + 100);
    assert_eq!(session.profile.inventory.items.len(), items_before + 1);
    assert_eq!(notes.len(), 2);

    // Flag now set: the reward rule no longer matches.
    assert_eq!(
        select_dialogue(&npc, &session).unwrap().lines[0],
        "fallback"
    );
}
