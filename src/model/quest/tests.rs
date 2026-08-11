use super::*;

#[test]
fn quest_runs_offer_to_reward() {
    let data = GameData::load().unwrap();
    let mut session = GameSession::new_game(&data);
    let id = "morning_thinning";
    let count = data.quests.get(id).unwrap().objective.count;

    assert!(session.quests.is_untaken(id));
    assert!(start(&mut session, &data, id).is_some());
    assert!(session.quests.is_active(id));
    // Starting again is a no-op.
    assert!(start(&mut session, &data, id).is_none());

    // One short of the goal: still active, not ready.
    advance_subdue(&mut session, &data, count - 1);
    assert!(session.quests.is_active(id));
    assert!(!session.quests.is_ready(id));

    // The step that meets the objective flips it to ready.
    let notes = advance_subdue(&mut session, &data, 1);
    assert!(!notes.is_empty());
    assert!(session.quests.is_ready(id));

    let scrip_before = session.profile.inventory.scrip;
    let reward = complete(&mut session, &data, id);
    assert!(!reward.is_empty());
    assert!(session.quests.is_done(id));
    assert!(session.profile.inventory.scrip > scrip_before);
}
