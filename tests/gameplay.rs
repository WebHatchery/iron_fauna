use iron_fauna::combat::engine::Battle;
use iron_fauna::combat::resolve;
use iron_fauna::combat::unit::{BattleUnit, UnitSpec};
use iron_fauna::combat::{
    BattleContext, BattleOutcome, BattleRewards, PlayerCommand, RiderMods, Side, Stance,
};
use iron_fauna::data::graftware::GraftKind;
use iron_fauna::data::species::{LimbArchetype, SizeClass, WeightClass};
use iron_fauna::data::world::{MapKind, TileKind};
use iron_fauna::data::GameData;
use iron_fauna::model::creature::{CreatureInstance, CreatureOrigin, EquipError};
use iron_fauna::model::duel::{self, PendingDuel};
use iron_fauna::model::inventory::{GraftCondition, GraftItem, Inventory};
use iron_fauna::model::party::{AcquireResult, Roster};
use iron_fauna::model::quest;
use iron_fauna::model::worldstate::Verdict;
use iron_fauna::state::{migrate_save_value, GameSession, Location, PaceSetting, SaveData};

fn spec(species: &str, side: Side, grafts: Vec<(&str, usize, &str)>) -> UnitSpec {
    UnitSpec {
        species_id: species.to_owned(),
        name: species.to_owned(),
        side,
        creature_id: None,
        bond: 0.0,
        stance: Stance::Aggressive,
        grafts: grafts
            .into_iter()
            .map(|(limb, slot, graft)| (limb.to_owned(), slot, graft.to_owned(), None))
            .collect(),
    }
}

#[test]
fn embedded_catalogs_and_balance_are_valid() {
    let data = GameData::load().unwrap();
    assert_eq!(data.config.game_name, "iron_fauna");
    assert_eq!(data.balance.party_slot_budget, 6);
    assert!(data.species.len() >= 4);
    assert!(data.graftware.len() >= 6);
    assert!(data.items.len() > 0);
    for (_, species) in data.species.iter() {
        assert!((1..=4).contains(&species.tier));
        assert!((species.power <= 100) && (species.speed <= 100));
        assert!(!species.limbs.is_empty());
        assert!(species.limbs.iter().all(|limb| limb.mounts.len() <= 2));
        if species.archetype == LimbArchetype::Flier {
            assert!(species.natural_flight);
            assert!(species.mount_count() <= 2);
        }
        let derived = species.derived(&data.balance);
        assert!(derived.core_hp > 0.0 && derived.limb_hp > 0.0);
    }
    for (_, graft) in data.graftware.iter() {
        assert!((1..=4).contains(&graft.tier));
        assert!(graft.power_draw > 0 && graft.value > 0);
        match graft.kind {
            GraftKind::Weapon => assert!(graft.damage > 0.0 && graft.cooldown > 0.0),
            GraftKind::Armor => assert!(graft.armor > 0.0 || graft.plating_hp > 0.0),
            GraftKind::Utility => assert!(graft.effect.is_some()),
        }
    }
}

#[test]
fn world_maps_and_factory_floors_are_consistent() {
    let data = GameData::load().unwrap();
    assert!(data.world.map(&data.world.start_map).is_some());
    for map in &data.world.maps {
        assert!(map.width() > 0 && map.height() > 0);
        assert!(map
            .rows
            .iter()
            .all(|row| row.chars().count() as i32 == map.width()));
        assert!(map.walkable(map.spawn_x, map.spawn_y));
        assert!(data.world.region(&map.region).is_some());
        for row in &map.rows {
            assert!(row.chars().all(|tile| TileKind::from_char(tile).is_some()));
        }
        for warp in &map.warps {
            let target = data.world.map(&warp.to_map).unwrap();
            assert!(target.walkable(warp.to_x, warp.to_y));
        }
        if map.encounter_rate > 0.0 {
            assert!(!map.encounters.is_empty());
        }
        for door in &map.doors {
            assert_eq!(map.tile(door.x, door.y), TileKind::SettlementDoor);
            assert!(map.settlement.is_some());
        }
    }
    for (_, factory) in data.factories.iter() {
        assert!(!factory.floors.is_empty());
        for floor in &factory.floors {
            let merged = data.world.map(&floor.id).unwrap();
            assert_eq!(merged.kind, MapKind::Factory);
            assert_eq!(merged.factory_id.as_deref(), Some(factory.id.as_str()));
        }
    }
}

#[test]
fn new_game_equips_starter_and_save_round_trips() {
    let data = GameData::load().unwrap();
    let session = GameSession::new_game(&data);
    assert_eq!(session.pace, PaceSetting::Wait);
    assert_eq!(session.profile.roster.party.len(), 1);
    assert!(!session
        .profile
        .roster
        .party_members()
        .next()
        .unwrap()
        .loadout
        .is_empty());

    let save = SaveData::from_session(&session, "0.1.0");
    let json = serde_json::to_value(&save).unwrap();
    let restored = migrate_save_value(None, json, "0.1.0").unwrap();
    assert_eq!(restored.session.location, session.location);
    assert_eq!(
        restored.session.profile.roster.creatures.len(),
        session.profile.roster.creatures.len()
    );
}

#[test]
fn creature_loadout_and_party_budget_enforce_rules() {
    let data = GameData::load().unwrap();
    let mut inventory = Inventory::new(100);
    inventory.items.push(GraftItem {
        id: 1,
        def_id: "spark_coil".to_owned(),
        condition: GraftCondition::Intact,
    });
    inventory.items.push(GraftItem {
        id: 2,
        def_id: "bolt_cannon".to_owned(),
        condition: GraftCondition::Intact,
    });
    let mut creature = CreatureInstance::new(10, "volpi", CreatureOrigin::Starter);
    creature
        .equip(&data, &inventory, "foreleg_l", 0, 1)
        .unwrap();
    assert!(matches!(
        creature.equip(&data, &inventory, "foreleg_r", 0, 2),
        Err(EquipError::TooHeavy)
    ));
    assert!(matches!(
        creature.equip(&data, &inventory, "foreleg_l", 0, 1),
        Err(EquipError::SlotOccupied)
    ));
    assert!(creature
        .boost_summary(&data, &inventory)
        .iter()
        .any(|line| line.contains("Spark Coil")));

    let mut roster = Roster::new();
    let large = |id| CreatureInstance::new(id, "ferrobruin", CreatureOrigin::Wild);
    assert_eq!(roster.acquire(&data, large(1)), AcquireResult::JoinedParty);
    assert_eq!(roster.acquire(&data, large(2)), AcquireResult::JoinedParty);
    assert_eq!(
        roster.acquire(
            &data,
            CreatureInstance::new(3, "volpi", CreatureOrigin::Wild)
        ),
        AcquireResult::SentToStorage
    );
    assert_eq!(roster.slots_free(&data), 0);
}

#[test]
fn quests_and_world_verdicts_follow_their_lifecycle() {
    let data = GameData::load().unwrap();
    let mut session = GameSession::new_game(&data);
    let quest_id = "morning_thinning";
    let goal = data.quests.get(quest_id).unwrap().objective.count;
    assert!(quest::start(&mut session, &data, quest_id).is_some());
    quest::advance_subdue(&mut session, &data, goal);
    assert!(session.quests.is_ready(quest_id));
    assert!(!quest::complete(&mut session, &data, quest_id).is_empty());
    assert!(session.quests.is_done(quest_id));

    for (id, _) in data.factories.iter() {
        session.world_state.factory_mut(id).verdict = Some(Verdict::Reseed);
    }
    assert!(session.world_state.all_judged(&data));
    assert_eq!(
        session.world_state.verdict_tally(&data).1,
        data.factories.len()
    );
}

#[test]
fn duel_rules_protect_equipped_parts_and_update_rank() {
    let data = GameData::load().unwrap();
    let mut session = GameSession::new_game(&data);
    let duelist = data
        .settlements
        .get("fernhollow")
        .unwrap()
        .duelist("bruk")
        .unwrap()
        .clone();
    let eligible = duel::eligible_stakes(&session, &data, 0);
    let equipped = session.profile.equipped_item_ids();
    assert!(eligible.iter().all(|id| !equipped.contains(id)));
    let pending = PendingDuel {
        settlement_id: "fernhollow".to_owned(),
        duelist_id: "bruk".to_owned(),
        my_stake: None,
    };
    duel::apply_duel_result(&mut session, &data, &pending, &duelist, true);
    assert_eq!(
        duel::current_rank(&session, "fernhollow"),
        duelist.rank_req + 1
    );
}

#[test]
fn battle_units_reject_bad_mounts_and_commands_respect_riding() {
    let data = GameData::load().unwrap();
    let unit = BattleUnit::build(
        &spec("volpi", Side::Player, vec![("foreleg_l", 0, "spark_coil")]),
        &data,
    )
    .unwrap();
    assert_eq!(unit.mounts.len(), 1);
    assert_eq!(unit.weapon_mounts(&data).len(), 1);
    assert!(unit.current_draw(&data) > 0.0);
    assert!(BattleUnit::build(
        &spec("volpi", Side::Player, vec![("foreleg_l", 0, "bolt_cannon")]),
        &data
    )
    .is_err());

    let mut battle = Battle::new(
        &data,
        BattleContext::WildSubdue,
        &[
            spec("volpi", Side::Player, vec![]),
            spec("pangol", Side::Player, vec![]),
        ],
        &[spec("bumblit", Side::Enemy, vec![])],
        RiderMods::neutral(),
        11,
    )
    .unwrap();
    assert!(battle.command(&data, PlayerCommand::BeginHop { to: 1 }));
    assert!(battle.ridden_unit().is_none());
    assert!(!battle.command(
        &data,
        PlayerCommand::NaturalAttack {
            target: 2,
            called: None
        }
    ));
    for _ in 0..40 {
        battle.update(&data, 0.05);
    }
    assert_eq!(battle.ridden_unit(), Some(1));
}

#[test]
fn battle_resolution_awards_capture_salvage_and_scrip() {
    let data = GameData::load().unwrap();
    let mut session = GameSession::new_game(&data);
    let creatures_before = session.profile.roster.creatures.len();
    let items_before = session.profile.inventory.items.len();
    let player_id = session.profile.roster.party.first().copied();
    let mut battle = Battle::new(
        &data,
        BattleContext::WildSubdue,
        &[UnitSpec {
            species_id: "volpi".to_owned(),
            name: "Volpi".to_owned(),
            side: Side::Player,
            creature_id: player_id,
            bond: 0.0,
            stance: Stance::Aggressive,
            grafts: vec![],
        }],
        &[spec("bumblit", Side::Enemy, vec![])],
        RiderMods::neutral(),
        1,
    )
    .unwrap();
    battle.outcome = Some(BattleOutcome::Victory(BattleRewards {
        captured_species: vec!["bumblit".to_owned()],
        salvage: vec!["spark_coil".to_owned()],
        scrip: 25,
    }));
    let summary = resolve::apply(&mut session, &data, &battle);
    assert_eq!(session.profile.roster.creatures.len(), creatures_before + 1);
    assert_eq!(session.profile.inventory.items.len(), items_before + 1);
    assert!(!summary.lines.is_empty());
}

#[test]
fn location_state_is_serializable_for_a_midgame_save() {
    let data = GameData::load().unwrap();
    let mut session = GameSession::new_game(&data);
    session.location = Location {
        map_id: data.world.start_map.clone(),
        x: 5,
        y: 6,
    };
    session.steps = 4213;
    let save = SaveData::from_session(&session, "0.1.0");
    let json = serde_json::to_value(save).unwrap();
    let restored = migrate_save_value(None, json, "0.1.0").unwrap().session;
    assert_eq!(restored.location, session.location);
    assert_eq!(restored.steps, 4213);
}

#[test]
fn weight_classes_and_size_costs_are_ordered() {
    assert!(WeightClass::Light < WeightClass::Medium);
    assert!(WeightClass::Medium < WeightClass::Heavy);
    assert_eq!(SizeClass::Small.slot_cost(), 1);
    assert_eq!(SizeClass::Huge.slot_cost(), 4);
}
