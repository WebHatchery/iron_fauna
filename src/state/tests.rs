use super::*;
use crate::data::GameData;

#[test]
fn new_game_arms_the_starter() {
    let data = GameData::load().unwrap();
    let session = GameSession::new_game(&data);
    assert_eq!(session.profile.roster.party.len(), 1);
    let starter = session.profile.roster.party_members().next().unwrap();
    assert_eq!(starter.species_id, "volpi");
    assert!(!starter.loadout.is_empty());
    assert_eq!(session.pace, PaceSetting::Wait);
}

#[test]
fn save_roundtrip_preserves_profile() {
    let data = GameData::load().unwrap();
    let session = GameSession::new_game(&data);
    let save = SaveData::from_session(&session, "0.1.0");
    let json = serde_json::to_value(&save).unwrap();
    let restored = migrate_save_value(Some("0.1.0".to_owned()), json, "0.1.0").unwrap();
    assert_eq!(
        restored.session.profile.roster.party.len(),
        session.profile.roster.party.len()
    );
}

#[test]
fn save_roundtrip_preserves_full_world_state() {
    use crate::model::rider::RiderUpgrade;
    use crate::model::worldstate::Verdict;

    let data = GameData::load().unwrap();
    let mut session = GameSession::new_game(&data);

    // Advance a realistic mid-late game: verdicts, relapse, story flags,
    // duel rank, rider upgrade, location, banked catches.
    session.world_state.factory_mut("the_cradle").heart_defeated = true;
    session.world_state.factory_mut("the_cradle").verdict = Some(Verdict::Reseed);
    session.world_state.factory_mut("the_cradle").invested = true;
    session.world_state.factory_mut("the_font").verdict = Some(Verdict::Bind);
    session.story_flags.insert("moel_gift".to_owned());
    session.story_flags.insert("bonewhite_truth1".to_owned());
    session
        .profile
        .rider
        .duel_ranks
        .insert("fernhollow".to_owned(), 2);
    session.profile.rider.grant(RiderUpgrade::SteadyHands);
    session.profile.spawn_creature(
        &data,
        "weavil",
        crate::model::creature::CreatureOrigin::Wild,
    );
    session.location = Location {
        map_id: "rot_causeway".to_owned(),
        x: 5,
        y: 6,
    };
    session.steps = 4213;

    let save = SaveData::from_session(&session, "0.1.0");
    let json = serde_json::to_value(&save).unwrap();
    let r = migrate_save_value(None, json, "0.1.0").unwrap().session;

    assert_eq!(r.world_state.verdict("the_cradle"), Some(Verdict::Reseed));
    assert!(r.world_state.factory("the_cradle").invested);
    assert_eq!(r.world_state.verdict("the_font"), Some(Verdict::Bind));
    assert!(r.story_flags.contains("moel_gift"));
    assert!(r.story_flags.contains("bonewhite_truth1"));
    assert_eq!(r.profile.rider.duel_ranks.get("fernhollow"), Some(&2));
    assert!(r.profile.rider.has(RiderUpgrade::SteadyHands));
    assert_eq!(r.location.map_id, "rot_causeway");
    assert_eq!(r.steps, 4213);
    assert_eq!(
        r.profile.roster.creatures.len(),
        session.profile.roster.creatures.len()
    );
}
