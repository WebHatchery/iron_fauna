use crate::combat::unit::{BattleUnit, UnitSpec};
use crate::combat::{Side, Stance};
use crate::data::GameData;

/// Every duelist party must actually assemble into legal battle units —
/// species exist, limbs exist, weight classes fit.
#[test]
fn settlements_are_valid() {
    let data = GameData::load().unwrap();
    assert!(!data.settlements.is_empty());

    for (_, s) in data.settlements.iter() {
        assert!(
            data.world.region(&s.region).is_some(),
            "{}: bad region",
            s.id
        );
        for entry in &s.shop {
            assert!(
                data.graftware.contains(&entry.graft),
                "{}: unknown shop graft {}",
                s.id,
                entry.graft
            );
        }
        for duelist in &s.duelists {
            assert!(!duelist.party.is_empty(), "{}: empty party", duelist.id);
            if !duelist.practice {
                let stake = duelist
                    .stake
                    .as_ref()
                    .unwrap_or_else(|| panic!("{}: staked duelist without a stake", duelist.id));
                assert!(
                    data.graftware.contains(stake),
                    "{}: unknown stake {}",
                    duelist.id,
                    stake
                );
                assert!(duelist.min_stake_value > 0, "{}: free stake", duelist.id);
            }
            for unit in &duelist.party {
                let spec = UnitSpec {
                    species_id: unit.species.clone(),
                    name: duelist.name.clone(),
                    side: Side::Enemy,
                    creature_id: None,
                    bond: 0.0,
                    stance: Stance::Aggressive,
                    grafts: unit
                        .grafts
                        .iter()
                        .map(|g| (g.limb.clone(), g.slot, g.graft.clone(), None))
                        .collect(),
                };
                BattleUnit::build(&spec, &data)
                    .unwrap_or_else(|err| panic!("{}: illegal duel unit: {}", duelist.id, err));
            }
        }
    }
}
