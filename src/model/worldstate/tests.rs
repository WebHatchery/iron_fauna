use super::*;
use crate::data::GameData;

#[test]
fn verdicts_reshape_the_region() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::default();
    assert_eq!(
        world.region_mood(&data, "verdant_hollow"),
        RegionMood::Threatened
    );
    assert!(world.factory_active("the_cradle"));

    world.factory_mut("the_cradle").heart_defeated = true;
    world.factory_mut("the_cradle").verdict = Some(Verdict::Purge);
    assert_eq!(world.region_mood(&data, "verdant_hollow"), RegionMood::Dead);
    assert!(!world.factory_active("the_cradle"));

    world.factory_mut("the_cradle").verdict = Some(Verdict::Reseed);
    assert_eq!(
        world.region_mood(&data, "verdant_hollow"),
        RegionMood::Reviving
    );
    assert!(
        WorldState::encounter_rate_mult(RegionMood::Reviving)
            > WorldState::encounter_rate_mult(RegionMood::Dead)
    );
}

#[test]
fn untended_reseed_relapses_and_investment_slows_it() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::default();
    world.factory_mut("the_cradle").verdict = Some(Verdict::Reseed);

    // Untended: ticks accumulate to relapse.
    let mut tipped = None;
    for _ in 0..5000 {
        if let Some(id) = world.tick_relapse(0.001, 0.15) {
            tipped = Some(id);
            break;
        }
    }
    assert_eq!(tipped.as_deref(), Some("the_cradle"));
    assert_eq!(
        world.region_mood(&data, "verdant_hollow"),
        RegionMood::Relapsed
    );

    // Confronted: back to reviving.
    let state = world.factory_mut("the_cradle");
    state.relapsed = false;
    state.relapse_risk = 0.0;
    assert_eq!(
        world.region_mood(&data, "verdant_hollow"),
        RegionMood::Reviving
    );

    // Invested: the same number of ticks doesn't tip.
    let mut watched = WorldState::default();
    watched.factory_mut("the_cradle").verdict = Some(Verdict::Reseed);
    watched.factory_mut("the_cradle").invested = true;
    for _ in 0..5000 {
        assert!(watched.tick_relapse(0.001, 0.15).is_none());
    }
    assert!(watched.factory("the_cradle").relapse_risk < 1.0);
}
