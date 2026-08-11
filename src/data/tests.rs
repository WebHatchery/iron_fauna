use super::*;

#[test]
fn embedded_data_loads() {
    let data = GameData::load().unwrap();
    assert_eq!(data.config.game_name, "iron_fauna");
    assert!(!data.species.is_empty());
    assert!(!data.graftware.is_empty());
}
