use crate::data::GameData;
use crate::state::GameSession;

#[test]
fn collection_tracks_owned_species() {
    let data = GameData::load().unwrap();
    let mut session = GameSession::new_game(&data);
    // Starter Volpi is owned; a never-caught species is not.
    assert!(session.profile.roster.owns_species("volpi"));
    assert!(!session.profile.roster.owns_species("tembolo"));
    session.profile.spawn_creature(
        &data,
        "tembolo",
        crate::model::creature::CreatureOrigin::Wild,
    );
    assert!(session.profile.roster.owns_species("tembolo"));
    assert_eq!(session.profile.roster.species_count("volpi"), 1);
}
