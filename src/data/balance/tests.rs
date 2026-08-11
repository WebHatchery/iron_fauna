use crate::data::GameData;

#[test]
fn balance_loads_with_sane_values() {
    let bal = GameData::load().unwrap().balance;
    assert_eq!(bal.party_slot_budget, 6);
    assert!(bal.vigor.ridden_regen_mult > 1.0);
    assert!(bal.battle.called_shot_accuracy_mult <= 1.0);
    assert!(bal.battle.weapon_damage_mult > 0.0);
    assert!(bal.battle.element_synergy_mult > 1.0);
}
