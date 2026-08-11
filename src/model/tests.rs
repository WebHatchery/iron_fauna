use super::*;
use crate::data::GameData;

#[test]
fn profile_spawns_creatures_with_unique_ids() {
    let data = GameData::load().unwrap();
    let mut profile = Profile::new("Ash", 200);
    let a = profile.spawn_creature(&data, "volpi", CreatureOrigin::Starter);
    let b = profile.spawn_creature(&data, "bumblit", CreatureOrigin::Wild);
    let item = profile.grant_graft("spark_coil", GraftCondition::Intact);
    assert_ne!(a, b);
    assert_ne!(b, item);
    assert_eq!(profile.roster.party.len(), 2);
}
