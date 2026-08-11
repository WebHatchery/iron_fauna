use super::*;
use crate::combat::engine::Battle;
use crate::combat::unit::UnitSpec;
use crate::combat::{BattleRewards, RiderMods, Stance};

#[test]
fn victory_grants_captures_salvage_and_scrip() {
    let data = GameData::load().unwrap();
    let mut session = GameSession::new_game(&data);
    let scrip_before = session.profile.inventory.scrip;
    let creatures_before = session.profile.roster.creatures.len();
    let items_before = session.profile.inventory.items.len();

    let specs = [UnitSpec {
        species_id: "volpi".to_owned(),
        name: "Volpi".to_owned(),
        side: Side::Player,
        creature_id: session.profile.roster.party.first().copied(),
        bond: 0.0,
        stance: Stance::Aggressive,
        grafts: vec![],
    }];
    let enemy = [UnitSpec {
        species_id: "bumblit".to_owned(),
        name: "Bumblit".to_owned(),
        side: Side::Enemy,
        creature_id: None,
        bond: 0.0,
        stance: Stance::Aggressive,
        grafts: vec![],
    }];
    let mut battle = Battle::new(
        &data,
        BattleContext::WildSubdue,
        &specs,
        &enemy,
        RiderMods::neutral(),
        1,
    )
    .unwrap();
    battle.outcome = Some(BattleOutcome::Victory(BattleRewards {
        captured_species: vec!["bumblit".to_owned()],
        salvage: vec!["spark_coil".to_owned()],
        scrip: 25,
    }));

    let summary = apply(&mut session, &data, &battle);
    assert_eq!(session.profile.roster.creatures.len(), creatures_before + 1);
    assert_eq!(session.profile.inventory.items.len(), items_before + 1);
    assert_eq!(session.profile.inventory.scrip, scrip_before + 25);
    assert!(!summary.lines.is_empty());
    assert_eq!(session.battles_fought, 1);
}
