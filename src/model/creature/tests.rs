use super::*;
use crate::data::GameData;
use crate::model::inventory::{GraftCondition, GraftItem};

fn setup() -> (GameData, Inventory, CreatureInstance) {
    let data = GameData::load().unwrap();
    let mut inv = Inventory::new(100);
    inv.items.push(GraftItem {
        id: 1,
        def_id: "spark_coil".to_owned(),
        condition: GraftCondition::Intact,
    });
    inv.items.push(GraftItem {
        id: 2,
        def_id: "bolt_cannon".to_owned(),
        condition: GraftCondition::Intact,
    });
    let creature = CreatureInstance::new(10, "volpi", CreatureOrigin::Starter);
    (data, inv, creature)
}

#[test]
fn equip_respects_weight_class_and_power_floor() {
    let (data, inv, mut volpi) = setup();
    // Light coil on a light mount: fine.
    volpi.equip(&data, &inv, "foreleg_l", 0, 1).unwrap();
    // Heavy cannon on a light mount: rejected for weight (and power).
    let err = volpi.equip(&data, &inv, "foreleg_r", 0, 2).unwrap_err();
    assert!(matches!(err, EquipError::TooHeavy));
    // Same slot twice: rejected.
    let err = volpi.equip(&data, &inv, "foreleg_l", 0, 1).unwrap_err();
    assert!(matches!(err, EquipError::SlotOccupied));
}

#[test]
fn boost_summary_reflects_equipped_graftware() {
    let (data, inv, mut volpi) = setup();
    // Nothing equipped: no special boost lines.
    assert!(volpi.boost_summary(&data, &inv).is_empty());
    // Spark coil carries a ChainArc boost — it should surface.
    volpi.equip(&data, &inv, "foreleg_l", 0, 1).unwrap();
    let summary = volpi.boost_summary(&data, &inv);
    assert_eq!(summary.len(), 1);
    assert!(summary[0].contains("Spark Coil"));
    assert!(summary[0].to_lowercase().contains("arc"));
}

#[test]
fn overdraw_is_positive_only_beyond_capacity() {
    let (data, inv, mut volpi) = setup();
    assert_eq!(volpi.overdraw(&data, &inv), 0.0);
    volpi.equip(&data, &inv, "foreleg_l", 0, 1).unwrap();
    // Volpi capacity = 40 * 1.6 = 64; spark coil draws 15 → no overdraw.
    assert_eq!(volpi.overdraw(&data, &inv), 0.0);
    assert!(volpi.total_power_draw(&data, &inv) > 0.0);
}
