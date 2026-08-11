use super::*;
use crate::data::GameData;
use crate::model::creature::CreatureOrigin;

fn creature(id: u64, species: &str) -> CreatureInstance {
    CreatureInstance::new(id, species, CreatureOrigin::Wild)
}

#[test]
fn party_budget_enforced_by_size() {
    let data = GameData::load().unwrap();
    let mut roster = Roster::new();

    // Two Larges (3 + 3) fill the budget of 6.
    assert_eq!(
        roster.acquire(&data, creature(1, "ferrobruin")),
        AcquireResult::JoinedParty
    );
    assert_eq!(
        roster.acquire(&data, creature(2, "ferrobruin")),
        AcquireResult::JoinedParty
    );
    assert_eq!(roster.slots_free(&data), 0);

    // A third catch banks to storage instead.
    assert_eq!(
        roster.acquire(&data, creature(3, "volpi")),
        AcquireResult::SentToStorage
    );
    assert_eq!(roster.stored().count(), 1);

    // Swapping a Large out frees room for the Small.
    assert!(roster.remove_from_party(2));
    assert!(roster.add_to_party(&data, 3));
    assert_eq!(roster.slots_used(&data), 4);
}
