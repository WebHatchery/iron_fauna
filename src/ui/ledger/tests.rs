use crate::data::GameData;
use crate::model::worldstate::Verdict;
use crate::state::GameSession;

#[test]
fn all_judged_only_when_every_factory_decided() {
    let data = GameData::load().unwrap();
    let mut session = GameSession::new_game(&data);
    assert!(!session.world_state.all_judged(&data));
    for (id, _) in data.factories.iter() {
        session.world_state.factory_mut(id).verdict = Some(Verdict::Reseed);
    }
    assert!(session.world_state.all_judged(&data));
    let (_, reseed, _) = session.world_state.verdict_tally(&data);
    assert_eq!(reseed, data.factories.len());
}
