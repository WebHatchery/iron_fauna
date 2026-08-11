use super::*;
use crate::data::GameData;

#[test]
fn consumables_are_valid() {
    let data = GameData::load().unwrap();
    assert!(!data.items.is_empty(), "no consumables authored");
    for (_, it) in data.items.iter() {
        assert!(it.value > 0, "{}: no value", it.id);
        if let ConsumableEffect::Ammo {
            magazine,
            damage_mult,
            ..
        } = it.effect
        {
            assert!(magazine > 0, "{}: empty magazine", it.id);
            assert!(damage_mult > 0.0, "{}: zero damage", it.id);
        }
    }
}
