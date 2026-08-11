use crate::data::GameData;

#[test]
fn world_is_internally_consistent() {
    let data = GameData::load().unwrap();
    let world = &data.world;
    assert!(world.map(&world.start_map).is_some(), "start map missing");

    for map in &world.maps {
        assert!(map.height() > 0 && map.width() > 0, "{}: empty map", map.id);
        for (y, row) in map.rows.iter().enumerate() {
            assert_eq!(
                row.chars().count() as i32,
                map.width(),
                "{}: ragged row {}",
                map.id,
                y
            );
            for (x, c) in row.chars().enumerate() {
                assert!(
                    crate::data::world::TileKind::from_char(c).is_some(),
                    "{}: unknown tile '{}' at {},{}",
                    map.id,
                    c,
                    x,
                    y
                );
            }
        }
        assert!(
            map.walkable(map.spawn_x, map.spawn_y),
            "{}: spawn blocked",
            map.id
        );
        assert!(
            world.region(&map.region).is_some(),
            "{}: bad region",
            map.id
        );
        for warp in &map.warps {
            let target = world.map(&warp.to_map);
            assert!(
                target.is_some(),
                "{}: warp to unknown map {}",
                map.id,
                warp.to_map
            );
            assert!(
                target.unwrap().walkable(warp.to_x, warp.to_y),
                "{}: warp lands blocked in {}",
                map.id,
                warp.to_map
            );
        }
        for door in &map.doors {
            assert_eq!(
                map.tile(door.x, door.y),
                super::TileKind::SettlementDoor,
                "{}: door at {},{} is not a settlement door tile",
                map.id,
                door.x,
                door.y
            );
            assert!(
                map.settlement.is_some(),
                "{}: has doors but no settlement",
                map.id
            );
        }
        for e in &map.encounters {
            assert!(
                data.species.contains(&e.species),
                "{}: unknown species {}",
                map.id,
                e.species
            );
            assert!(e.min >= 1 && e.min <= e.max, "{}: bad pack bounds", map.id);
        }
        if map.encounter_rate > 0.0 {
            assert!(!map.encounters.is_empty(), "{}: rate but no table", map.id);
        }
        for npc in &map.npcs {
            assert!(
                !npc.lines.is_empty() || !npc.dialogue.is_empty(),
                "{}: NPC {} has nothing to say",
                map.id,
                npc.id
            );
            for rule in &npc.dialogue {
                assert!(!rule.lines.is_empty(), "{}: empty rule", npc.id);
                for def in &rule.give_grafts {
                    assert!(
                        data.graftware.contains(def),
                        "{}: unknown reward graft {}",
                        npc.id,
                        def
                    );
                }
                for q in [&rule.start_quest, &rule.complete_quest]
                    .into_iter()
                    .flatten()
                {
                    assert!(data.quests.contains(q), "{}: unknown quest {}", npc.id, q);
                }
                if let Some(cond) = &rule.when {
                    for q in [
                        &cond.quest_active,
                        &cond.quest_ready,
                        &cond.quest_done,
                        &cond.quest_none,
                    ]
                    .into_iter()
                    .flatten()
                    {
                        assert!(data.quests.contains(q), "{}: unknown quest {}", npc.id, q);
                    }
                }
                if let Some(cond) = &rule.when {
                    if let Some((factory, verdict)) = &cond.verdict {
                        assert!(
                            data.factories.contains(factory),
                            "{}: unknown factory {}",
                            npc.id,
                            factory
                        );
                        assert!(
                            matches!(verdict.as_str(), "Purge" | "Reseed" | "Bind"),
                            "{}: bad verdict name {}",
                            npc.id,
                            verdict
                        );
                    }
                    if let Some(factory) = &cond.heart_defeated {
                        assert!(
                            data.factories.contains(factory),
                            "{}: unknown factory {}",
                            npc.id,
                            factory
                        );
                    }
                    if let Some((factory, _)) = &cond.relapsed {
                        assert!(
                            data.factories.contains(factory),
                            "{}: unknown factory {}",
                            npc.id,
                            factory
                        );
                    }
                }
            }
        }
    }
}
