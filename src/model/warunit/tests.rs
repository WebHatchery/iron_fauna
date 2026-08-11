use super::*;
use crate::combat::unit::{BattleUnit, UnitSpec};
use crate::combat::{Side, Stance};
use crate::data::GameData;

#[test]
fn war_units_are_always_legal_and_deterministic() {
    let data = GameData::load().unwrap();
    for (_, species) in data.species.iter() {
        let mut rng_a = Rng::new(31 + species.power as u64);
        let mut rng_b = Rng::new(31 + species.power as u64);
        let grafts = war_unit_grafts(species, &data, species.tier.max(2), &mut rng_a);
        let again = war_unit_grafts(species, &data, species.tier.max(2), &mut rng_b);
        assert_eq!(grafts, again, "{}: nondeterministic outfit", species.id);

        let spec = UnitSpec {
            species_id: species.id.clone(),
            name: species.name.clone(),
            side: Side::Enemy,
            creature_id: None,
            bond: 0.0,
            stance: Stance::Aggressive,
            grafts,
        };
        BattleUnit::build(&spec, &data)
            .unwrap_or_else(|err| panic!("{}: illegal war unit: {}", species.id, err));
    }
}
