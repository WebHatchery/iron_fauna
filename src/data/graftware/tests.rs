use super::*;
use crate::data::GameData;

#[test]
fn catalog_is_valid() {
    let data = GameData::load().unwrap();
    assert!(
        data.graftware.len() >= 6,
        "expected at least seed catalog, found {}",
        data.graftware.len()
    );

    for (_, g) in data.graftware.iter() {
        assert!((1..=4).contains(&g.tier), "{}: bad tier", g.id);
        assert!(g.power_draw > 0, "{}: free graftware", g.id);
        assert!(g.value > 0, "{}: no value", g.id);
        match g.kind {
            GraftKind::Weapon => {
                assert!(g.damage > 0.0, "{}: weapon without damage", g.id);
                assert!(g.cooldown > 0.0, "{}: weapon without cooldown", g.id);
                assert!(g.vigor_cost > 0.0, "{}: weapon fires for free", g.id);
            }
            GraftKind::Armor => {
                assert!(
                    g.armor > 0.0 || g.plating_hp > 0.0,
                    "{}: armor without protection",
                    g.id
                );
            }
            GraftKind::Utility => {
                assert!(g.effect.is_some(), "{}: utility without effect", g.id);
            }
        }
    }
}

#[test]
fn heavier_tiers_draw_more_power_on_average() {
    let data = GameData::load().unwrap();
    let avg_draw = |w: WeightClass| {
        let (sum, n) = data
            .graftware
            .iter()
            .filter(|(_, g)| g.weight == w)
            .fold((0u32, 0u32), |(s, n), (_, g)| (s + g.power_draw, n + 1));
        if n == 0 {
            0.0
        } else {
            sum as f32 / n as f32
        }
    };
    let light = avg_draw(WeightClass::Light);
    let heavy = avg_draw(WeightClass::Heavy);
    if light > 0.0 && heavy > 0.0 {
        assert!(heavy > light, "heavy grafts should out-draw light ones");
    }
}
