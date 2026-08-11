use super::*;
use crate::combat::{Side, Stance};
use crate::data::GameData;

pub fn spec(species: &str, side: Side, grafts: Vec<(&str, usize, &str)>) -> UnitSpec {
    UnitSpec {
        species_id: species.to_owned(),
        name: species.to_owned(),
        side,
        creature_id: None,
        bond: 0.0,
        stance: Stance::Aggressive,
        grafts: grafts
            .into_iter()
            .map(|(l, s, d)| (l.to_owned(), s, d.to_owned(), None))
            .collect(),
    }
}

#[test]
fn build_wires_limbs_and_mounts() {
    let data = GameData::load().unwrap();
    let unit = BattleUnit::build(
        &spec("volpi", Side::Player, vec![("foreleg_l", 0, "spark_coil")]),
        &data,
    )
    .unwrap();
    assert_eq!(unit.limbs.len(), 5);
    assert_eq!(unit.mounts.len(), 1);
    assert!(!unit.core_exposed());
    assert_eq!(unit.weapon_mounts(&data).len(), 1);
    assert!(unit.current_draw(&data) > 0.0);
}

#[test]
fn heavy_graft_on_light_mount_is_rejected() {
    let data = GameData::load().unwrap();
    let err = BattleUnit::build(
        &spec("volpi", Side::Player, vec![("foreleg_l", 0, "bolt_cannon")]),
        &data,
    );
    assert!(err.is_err());
}

#[test]
fn core_exposed_when_all_limbs_severed() {
    let data = GameData::load().unwrap();
    let mut unit = BattleUnit::build(&spec("volpi", Side::Enemy, vec![]), &data).unwrap();
    for limb in &mut unit.limbs {
        limb.severed = true;
    }
    assert!(unit.core_exposed());
    assert!(unit.intact_limbs().is_empty());
}
