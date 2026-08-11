use super::*;
use crate::data::GameData;

#[test]
fn repair_costs_scrip_and_restores_condition() {
    let data = GameData::load().unwrap();
    let mut inv = Inventory::new(1000);
    inv.add(1, "spark_coil", GraftCondition::Damaged);

    assert!(inv.repair(&data, 1));
    assert_eq!(inv.item(1).unwrap().condition, GraftCondition::Intact);
    assert!(inv.scrip < 1000);

    // Repairing an intact item is a no-op failure.
    assert!(!inv.repair(&data, 1));
}

#[test]
fn repair_fails_when_broke() {
    let data = GameData::load().unwrap();
    let mut inv = Inventory::new(0);
    inv.add(1, "bolt_cannon", GraftCondition::Damaged);
    assert!(!inv.repair(&data, 1));
    assert_eq!(inv.item(1).unwrap().condition, GraftCondition::Damaged);
}
