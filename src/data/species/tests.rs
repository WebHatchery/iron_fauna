use super::*;
use crate::data::GameData;

#[test]
fn roster_is_valid() {
    let data = GameData::load().unwrap();
    let bal = &data.balance;
    assert!(
        data.species.len() >= 4,
        "expected at least seed roster, found {}",
        data.species.len()
    );

    for (_, sp) in data.species.iter() {
        assert!(sp.power <= 100, "{}: power out of range", sp.id);
        assert!(sp.speed <= 100, "{}: speed out of range", sp.id);
        assert!((1..=4).contains(&sp.tier), "{}: bad tier", sp.id);
        assert!(!sp.limbs.is_empty(), "{}: no limbs", sp.id);
        let mut limb_ids: Vec<&str> = sp.limbs.iter().map(|l| l.id.as_str()).collect();
        limb_ids.sort_unstable();
        limb_ids.dedup();
        assert_eq!(
            limb_ids.len(),
            sp.limbs.len(),
            "{}: duplicate limb ids",
            sp.id
        );
        for limb in &sp.limbs {
            assert!(
                limb.mounts.len() <= 2,
                "{}: limb {} has >2 mounts",
                sp.id,
                limb.id
            );
        }
        // Fliers trade mounts for flight (creature.md §2.4).
        if sp.archetype == LimbArchetype::Flier {
            assert!(sp.natural_flight, "{}: flier without flight", sp.id);
            assert!(
                sp.mount_count() <= 2,
                "{}: flier with too many mounts",
                sp.id
            );
        }

        let d = sp.derived(bal);
        assert!(d.vigor_max > 0.0 && d.core_hp > 0.0 && d.limb_hp > 0.0);
        assert!(d.strain_threshold > 0.0);
        assert!(d.power_capacity > 0.0);
    }
}

#[test]
fn gentle_temperament_lowers_strain_threshold() {
    let data = GameData::load().unwrap();
    let mult = &data.balance.curves.temperament_strain_mult;
    assert!(mult.gentle < mult.fierce);
}
