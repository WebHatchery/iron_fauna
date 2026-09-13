//! Semantic validation for merged game content.

use super::balance::BalanceConfig;
use super::graftware::GraftKind;
use super::item::ConsumableEffect;
use super::species::LimbArchetype;
use super::{GameConfig, GameData};
use std::collections::HashSet;

mod factories;

pub(super) fn validate(data: &GameData) -> Result<(), String> {
    validate_config(&data.config)?;
    validate_starter(data)?;
    validate_balance(&data.balance)?;
    validate_species(data)?;
    validate_graftware(data)?;
    validate_items(data)?;
    validate_quests(data)?;
    validate_world(data)?;
    validate_settlements(data)?;
    factories::validate_factories(data)?;
    factories::validate_textures(&data.texture_manifest)
}

fn validate_config(config: &GameConfig) -> Result<(), String> {
    for (field, value) in [
        ("game_name", config.game_name.as_str()),
        ("display_name", config.display_name.as_str()),
        ("save_slot", config.save_slot.as_str()),
        ("version", config.version.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("content validation: game_config.{field} is empty"));
        }
    }
    Ok(())
}

fn validate_starter(data: &GameData) -> Result<(), String> {
    let starter = &data.config.starter;
    if starter.rider_name.trim().is_empty()
        || starter.species_id.trim().is_empty()
        || starter.starting_scrip < 0
        || starter.journal_entry.trim().is_empty()
    {
        return Err(
            "content validation: starter identity, scrip, or journal entry is invalid".to_owned(),
        );
    }
    if !data.species.contains(&starter.species_id) {
        return Err(format!(
            "content validation: starter references unknown species '{}'",
            starter.species_id
        ));
    }
    for graft in &starter.initial_grafts {
        if !data.graftware.contains(graft) {
            return Err(format!(
                "content validation: starter references unknown graft '{}'",
                graft
            ));
        }
    }
    for consumable in &starter.consumables {
        if consumable.count == 0 || !data.items.contains(&consumable.item_id) {
            return Err(format!(
                "content validation: starter has invalid consumable '{}'",
                consumable.item_id
            ));
        }
    }
    let species = data
        .species
        .get(&starter.species_id)
        .expect("validated starter species");
    for equipment in &starter.equipment {
        if !starter.initial_grafts.contains(&equipment.graft_id) {
            return Err(format!(
                "content validation: starter equipment '{}' is not in initial_grafts",
                equipment.graft_id
            ));
        }
        let limb = species.limb(&equipment.limb_id).ok_or_else(|| {
            format!(
                "content validation: starter equipment references unknown limb '{}'",
                equipment.limb_id
            )
        })?;
        let mount = limb.mounts.get(equipment.slot).ok_or_else(|| {
            format!(
                "content validation: starter equipment references missing mount '{}'/{}",
                equipment.limb_id, equipment.slot
            )
        })?;
        let graft = data
            .graftware
            .get(&equipment.graft_id)
            .expect("validated starter graft");
        if graft.weight > *mount || species.power < graft.min_power {
            return Err(format!(
                "content validation: starter equipment '{}' cannot fit '{}'/{}",
                equipment.graft_id, equipment.limb_id, equipment.slot
            ));
        }
    }
    Ok(())
}

fn positive(label: &str, value: f32) -> Result<(), String> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(format!(
            "content validation: {label} must be finite and > 0, got {value}"
        ))
    }
}

fn non_negative(label: &str, value: f32) -> Result<(), String> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(format!(
            "content validation: {label} must be finite and >= 0, got {value}"
        ))
    }
}

fn finite(label: &str, value: f32) -> Result<(), String> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(format!(
            "content validation: {label} must be finite, got {value}"
        ))
    }
}

fn fraction(label: &str, value: f32) -> Result<(), String> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(format!(
            "content validation: {label} must be in 0..=1, got {value}"
        ))
    }
}

fn validate_balance(balance: &BalanceConfig) -> Result<(), String> {
    if balance.party_slot_budget == 0 {
        return Err("content validation: balance.party_slot_budget must be > 0".to_owned());
    }
    for (name, value) in [
        ("small", balance.party_slot_cost.small),
        ("medium", balance.party_slot_cost.medium),
        ("large", balance.party_slot_cost.large),
        ("huge", balance.party_slot_cost.huge),
    ] {
        if value == 0 || value > balance.party_slot_budget {
            return Err(format!(
                "content validation: balance.party_slot_cost.{name} must fit the party budget"
            ));
        }
    }
    let budget = &balance.power_budget;
    for (name, value) in [
        ("total", budget.total),
        ("power_per_point", budget.power_per_point),
        ("speed_per_point", budget.speed_per_point),
        ("extra_limb_cost", budget.extra_limb_cost),
        ("natural_flight_cost", budget.natural_flight_cost),
        ("innate_armor_per_point", budget.innate_armor_per_point),
        ("size_cost.small", budget.size_cost.small),
        ("size_cost.medium", budget.size_cost.medium),
        ("size_cost.large", budget.size_cost.large),
        ("size_cost.huge", budget.size_cost.huge),
    ] {
        non_negative(&format!("balance.power_budget.{name}"), value)?;
    }
    positive("balance.power_budget.total", budget.total)?;
    if balance.party_slot_budget == 0 {
        return Err("content validation: balance.party_slot_budget must be > 0".to_owned());
    }
    let c = &balance.curves;
    for (name, value) in [
        ("power_capacity_mult", c.power_capacity_mult),
        ("vigor_base", c.vigor_base),
        ("vigor_per_power", c.vigor_per_power),
        ("vigor_regen_base", c.vigor_regen_base),
        ("vigor_regen_per_power", c.vigor_regen_per_power),
        ("strain_base", c.strain_base),
        ("strain_per_power", c.strain_per_power),
        ("limb_hp_base", c.limb_hp_base),
        ("limb_hp_per_power", c.limb_hp_per_power),
        ("core_hp_base", c.core_hp_base),
        ("core_hp_per_power", c.core_hp_per_power),
        ("regrow_base", c.regrow_base),
        ("regrow_per_power", c.regrow_per_power),
        ("dodge_per_speed", c.dodge_per_speed),
        ("accuracy_slow_pivot", c.accuracy_slow_pivot),
    ] {
        positive(&format!("balance.curves.{name}"), value)?;
    }
    for (name, value) in [
        ("vigor_size_bonus.small", c.vigor_size_bonus.small),
        ("vigor_size_bonus.medium", c.vigor_size_bonus.medium),
        ("vigor_size_bonus.large", c.vigor_size_bonus.large),
        ("vigor_size_bonus.huge", c.vigor_size_bonus.huge),
    ] {
        non_negative(&format!("balance.curves.{name}"), value)?;
    }
    for (name, value) in [
        ("size_dodge_bonus.small", c.size_dodge_bonus.small),
        ("size_dodge_bonus.medium", c.size_dodge_bonus.medium),
        ("size_dodge_bonus.large", c.size_dodge_bonus.large),
        ("size_dodge_bonus.huge", c.size_dodge_bonus.huge),
    ] {
        finite(&format!("balance.curves.{name}"), value)?;
    }
    for (name, value) in [
        ("limb_hp_size_mult.small", c.limb_hp_size_mult.small),
        ("limb_hp_size_mult.medium", c.limb_hp_size_mult.medium),
        ("limb_hp_size_mult.large", c.limb_hp_size_mult.large),
        ("limb_hp_size_mult.huge", c.limb_hp_size_mult.huge),
        (
            "temperament_strain_mult.gentle",
            c.temperament_strain_mult.gentle,
        ),
        (
            "temperament_strain_mult.placid",
            c.temperament_strain_mult.placid,
        ),
        (
            "temperament_strain_mult.wary",
            c.temperament_strain_mult.wary,
        ),
        (
            "temperament_strain_mult.fierce",
            c.temperament_strain_mult.fierce,
        ),
    ] {
        positive(&format!("balance.curves.{name}"), value)?;
    }
    let v = &balance.vigor;
    for (name, value) in [
        ("ridden_regen_mult", v.ridden_regen_mult),
        ("regrow_cost_per_hp", v.regrow_cost_per_hp),
        ("reinforce_cost", v.reinforce_cost),
        ("reinforce_shield", v.reinforce_shield),
        ("reinforce_cooldown", v.reinforce_cooldown),
        ("reinforce_shield_cap_frac", v.reinforce_shield_cap_frac),
    ] {
        positive(&format!("balance.vigor.{name}"), value)?;
    }
    fraction(
        "balance.vigor.reinforce_shield_cap_frac",
        v.reinforce_shield_cap_frac,
    )?;
    let s = &balance.strain;
    for (name, value) in [
        ("ridden_gain_per_sec", s.ridden_gain_per_sec),
        ("overdraw_gain_per_point", s.overdraw_gain_per_point),
        ("fire_gain_per_draw", s.fire_gain_per_draw),
        ("calm_decay_per_sec", s.calm_decay_per_sec),
        ("accuracy_penalty_at_max", s.accuracy_penalty_at_max),
        ("regrow_penalty_at_max", s.regrow_penalty_at_max),
        ("berserk_duration", s.berserk_duration),
        ("berserk_chance", s.berserk_chance),
        ("rejection_relief_frac", s.rejection_relief_frac),
    ] {
        non_negative(&format!("balance.strain.{name}"), value)?;
    }
    fraction("balance.strain.berserk_chance", s.berserk_chance)?;
    fraction(
        "balance.strain.rejection_relief_frac",
        s.rejection_relief_frac,
    )?;
    let b = &balance.battle;
    for (name, value) in [
        ("called_shot_accuracy_mult", b.called_shot_accuracy_mult),
        ("base_accuracy", b.base_accuracy),
        ("salvage_drop_chance", b.salvage_drop_chance),
        ("hop_transit_time", b.hop_transit_time),
        ("exposed_core_damage_mult", b.exposed_core_damage_mult),
        ("weapon_damage_mult", b.weapon_damage_mult),
        ("graft_spill_frac", b.graft_spill_frac),
        ("natural_attack_per_power", b.natural_attack_per_power),
        ("natural_attack_cooldown", b.natural_attack_cooldown),
        ("element_synergy_mult", b.element_synergy_mult),
    ] {
        positive(&format!("balance.battle.{name}"), value)?;
    }
    fraction(
        "balance.battle.called_shot_accuracy_mult",
        b.called_shot_accuracy_mult,
    )?;
    fraction("balance.battle.salvage_drop_chance", b.salvage_drop_chance)?;
    fraction("balance.battle.graft_spill_frac", b.graft_spill_frac)?;
    non_negative(
        "balance.world.relapse_per_step",
        balance.world.relapse_per_step,
    )?;
    positive(
        "balance.world.relapse_invested_mult",
        balance.world.relapse_invested_mult,
    )?;
    if balance.world.watch_cost <= 0 {
        return Err(format!(
            "content validation: balance.world.watch_cost must be > 0, got {}",
            balance.world.watch_cost
        ));
    }
    Ok(())
}

fn validate_species(data: &GameData) -> Result<(), String> {
    for (id, species) in data.species.iter() {
        if species.id != *id || species.id.trim().is_empty() {
            return Err(format!(
                "content validation: species key '{id}' disagrees with its id"
            ));
        }
        if species.power > 100 || species.speed > 100 || !(1..=4).contains(&species.tier) {
            return Err(format!(
                "content validation: species '{id}' has invalid power, speed, or tier"
            ));
        }
        let budget_cost = species.power_budget_cost(&data.balance);
        if budget_cost > data.balance.power_budget.total + 0.001 {
            return Err(format!(
                "content validation: species '{id}' exceeds chassis power budget ({budget_cost:.1}/{:.1})",
                data.balance.power_budget.total
            ));
        }
        if species.limbs.is_empty() {
            return Err(format!("content validation: species '{id}' has no limbs"));
        }
        let mut limb_ids = HashSet::new();
        for limb in &species.limbs {
            if limb.id.trim().is_empty() || !limb_ids.insert(limb.id.as_str()) {
                return Err(format!(
                    "content validation: species '{id}' has duplicate or empty limb id"
                ));
            }
            if limb.mounts.len() > 2 {
                return Err(format!(
                    "content validation: species '{id}' limb '{}' has over two mounts",
                    limb.id
                ));
            }
        }
        if species.archetype == LimbArchetype::Flier
            && (!species.natural_flight || species.mount_count() > 2)
        {
            return Err(format!(
                "content validation: flier species '{id}' has invalid flight or mount count"
            ));
        }
        let derived = species.derived(&data.balance);
        for (name, value) in [
            ("power capacity", derived.power_capacity),
            ("vigor", derived.vigor_max),
            ("limb hp", derived.limb_hp),
            ("core hp", derived.core_hp),
            ("strain threshold", derived.strain_threshold),
        ] {
            positive(&format!("species '{id}': derived {name}"), value)?;
        }
    }
    Ok(())
}

fn validate_graftware(data: &GameData) -> Result<(), String> {
    for (id, graft) in data.graftware.iter() {
        if graft.id != *id
            || !(1..=4).contains(&graft.tier)
            || graft.power_draw == 0
            || graft.value <= 0
        {
            return Err(format!(
                "content validation: graftware '{id}' has invalid identity or balance"
            ));
        }
        match graft.kind {
            GraftKind::Weapon
                if graft.damage <= 0.0 || graft.cooldown <= 0.0 || graft.vigor_cost <= 0.0 =>
            {
                return Err(format!(
                    "content validation: weapon '{id}' has invalid damage, cooldown, or vigor cost"
                ));
            }
            GraftKind::Armor if graft.armor <= 0.0 && graft.plating_hp <= 0.0 => {
                return Err(format!(
                    "content validation: armor '{id}' has no protection"
                ));
            }
            GraftKind::Utility if graft.effect.is_none() => {
                return Err(format!("content validation: utility '{id}' has no effect"));
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_items(data: &GameData) -> Result<(), String> {
    for (id, item) in data.items.iter() {
        if item.id != *id || item.value <= 0 || item.name.trim().is_empty() {
            return Err(format!(
                "content validation: item '{id}' has invalid identity or value"
            ));
        }
        if let ConsumableEffect::Ammo {
            magazine,
            damage_mult,
            ..
        } = item.effect
        {
            if magazine == 0 || damage_mult <= 0.0 || !damage_mult.is_finite() {
                return Err(format!(
                    "content validation: ammo item '{id}' has invalid magazine or damage"
                ));
            }
        }
    }
    Ok(())
}

fn validate_quests(data: &GameData) -> Result<(), String> {
    for (id, quest) in data.quests.iter() {
        if quest.id != *id
            || quest.giver.trim().is_empty()
            || quest.tracker.trim().is_empty()
            || quest.objective.count == 0
        {
            return Err(format!(
                "content validation: quest '{id}' has incomplete identity or objective"
            ));
        }
        if quest.reward_scrip < 0 {
            return Err(format!(
                "content validation: quest '{id}' has negative scrip reward"
            ));
        }
        if let Some(region) = &quest.objective.region {
            if !data.world.regions.iter().any(|entry| entry.id == *region) {
                return Err(format!(
                    "content validation: quest '{id}' references unknown region '{region}'"
                ));
            }
        }
        for graft in &quest.reward_grafts {
            if !data.graftware.contains(graft) {
                return Err(format!(
                    "content validation: quest '{id}' references unknown graftware '{graft}'"
                ));
            }
        }
    }
    Ok(())
}

fn map_coordinate(map: &super::world::MapDef, x: i32, y: i32, field: &str) -> Result<(), String> {
    if x < 0 || y < 0 || x >= map.width() || y >= map.height() {
        return Err(format!(
            "content validation: map '{}' {field} coordinate ({x},{y}) is outside {}x{}",
            map.id,
            map.width(),
            map.height()
        ));
    }
    Ok(())
}

fn validate_world(data: &GameData) -> Result<(), String> {
    let mut region_ids = HashSet::new();
    for region in &data.world.regions {
        if region.id.trim().is_empty() || !region_ids.insert(region.id.as_str()) {
            return Err(format!(
                "content validation: duplicate or empty region id '{}'",
                region.id
            ));
        }
        if !(1..=4).contains(&region.tier) || !data.factories.contains(&region.gestarium_id) {
            return Err(format!(
                "content validation: region '{}' has invalid tier or factory '{}'",
                region.id, region.gestarium_id
            ));
        }
    }
    if data.world.map(&data.world.start_map).is_none() {
        return Err(format!(
            "content validation: start map '{}' does not exist",
            data.world.start_map
        ));
    }
    let mut map_ids = HashSet::new();
    for map in &data.world.maps {
        if map.id.trim().is_empty() || !map_ids.insert(map.id.as_str()) {
            return Err(format!(
                "content validation: duplicate or empty map id '{}'",
                map.id
            ));
        }
        if map.rows.is_empty()
            || map
                .rows
                .iter()
                .any(|row| row.chars().count() != map.width() as usize)
        {
            return Err(format!(
                "content validation: map '{}' has empty or ragged rows",
                map.id
            ));
        }
        for row in &map.rows {
            if row
                .chars()
                .any(|tile| super::world::TileKind::from_char(tile).is_none())
            {
                return Err(format!(
                    "content validation: map '{}' has an unknown tile",
                    map.id
                ));
            }
        }
        map_coordinate(map, map.spawn_x, map.spawn_y, "spawn")?;
        if !map.walkable(map.spawn_x, map.spawn_y) {
            return Err(format!(
                "content validation: map '{}' spawn is blocked",
                map.id
            ));
        }
        if data.world.region(&map.region).is_none() {
            return Err(format!(
                "content validation: map '{}' references unknown region '{}'",
                map.id, map.region
            ));
        }
        if let Some(settlement) = &map.settlement {
            if !data.settlements.contains(settlement) {
                return Err(format!(
                    "content validation: map '{}' references unknown settlement '{}'",
                    map.id, settlement
                ));
            }
        }
        for warp in &map.warps {
            map_coordinate(map, warp.x, warp.y, "warp source")?;
            let target = data.world.map(&warp.to_map).ok_or_else(|| {
                format!(
                    "content validation: map '{}' warps to unknown map '{}'",
                    map.id, warp.to_map
                )
            })?;
            map_coordinate(target, warp.to_x, warp.to_y, "warp destination")?;
            if !target.walkable(warp.to_x, warp.to_y) {
                return Err(format!(
                    "content validation: map '{}' warp lands on blocked tile in '{}'",
                    map.id, warp.to_map
                ));
            }
        }
        for door in &map.doors {
            map_coordinate(map, door.x, door.y, "door")?;
            if map.tile(door.x, door.y) != super::world::TileKind::SettlementDoor
                || map.settlement.is_none()
            {
                return Err(format!("content validation: map '{}' has a settlement door without a matching tile and settlement", map.id));
            }
        }
        for encounter in &map.encounters {
            if encounter.weight == 0
                || encounter.min == 0
                || encounter.min > encounter.max
                || !data.species.contains(&encounter.species)
            {
                return Err(format!(
                    "content validation: map '{}' has invalid encounter '{}'",
                    map.id, encounter.species
                ));
            }
        }
        if map.encounter_rate < 0.0 || !map.encounter_rate.is_finite() {
            return Err(format!(
                "content validation: map '{}' has invalid encounter rate",
                map.id
            ));
        }
        for npc in &map.npcs {
            map_coordinate(map, npc.x, npc.y, "NPC")?;
            if npc.lines.is_empty() && npc.dialogue.is_empty() {
                return Err(format!(
                    "content validation: NPC '{}' has no dialogue",
                    npc.id
                ));
            }
            for rule in &npc.dialogue {
                if rule.lines.is_empty() {
                    return Err(format!(
                        "content validation: NPC '{}' has an empty dialogue rule",
                        npc.id
                    ));
                }
                for graft in &rule.give_grafts {
                    if !data.graftware.contains(graft) {
                        return Err(format!(
                            "content validation: NPC '{}' references unknown graft '{}'",
                            npc.id, graft
                        ));
                    }
                }
                for quest in [&rule.start_quest, &rule.complete_quest]
                    .into_iter()
                    .flatten()
                {
                    if !data.quests.contains(quest) {
                        return Err(format!(
                            "content validation: NPC '{}' references unknown quest '{}'",
                            npc.id, quest
                        ));
                    }
                }
                if let Some(condition) = &rule.when {
                    for quest in [
                        &condition.quest_active,
                        &condition.quest_ready,
                        &condition.quest_done,
                        &condition.quest_none,
                    ]
                    .into_iter()
                    .flatten()
                    {
                        if !data.quests.contains(quest) {
                            return Err(format!("content validation: NPC '{}' condition references unknown quest '{}'", npc.id, quest));
                        }
                    }
                    if let Some((factory, verdict)) = &condition.verdict {
                        if !data.factories.contains(factory)
                            || !matches!(verdict.as_str(), "Purge" | "Reseed" | "Bind")
                        {
                            return Err(format!(
                                "content validation: NPC '{}' has an invalid verdict condition",
                                npc.id
                            ));
                        }
                    }
                    if let Some((factory, _)) = &condition.relapsed {
                        if !data.factories.contains(factory) {
                            return Err(format!("content validation: NPC '{}' condition references unknown factory '{}'", npc.id, factory));
                        }
                    }
                    if let Some(factory) = &condition.heart_defeated {
                        if !data.factories.contains(factory) {
                            return Err(format!("content validation: NPC '{}' condition references unknown factory '{}'", npc.id, factory));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_settlements(data: &GameData) -> Result<(), String> {
    for (id, settlement) in data.settlements.iter() {
        if settlement.id != *id || data.world.region(&settlement.region).is_none() {
            return Err(format!(
                "content validation: settlement '{id}' has invalid identity or region"
            ));
        }
        for entry in &settlement.shop {
            if !data.graftware.contains(&entry.graft) || entry.price.is_some_and(|price| price <= 0)
            {
                return Err(format!(
                    "content validation: settlement '{id}' has invalid shop entry '{}'",
                    entry.graft
                ));
            }
        }
        for duelist in &settlement.duelists {
            if duelist.party.is_empty()
                || duelist.reward_scrip < 0
                || (!duelist.practice && duelist.stake.is_none())
            {
                return Err(format!(
                    "content validation: duelist '{}' in '{id}' has incomplete setup",
                    duelist.id
                ));
            }
            if let Some(stake) = &duelist.stake {
                if !data.graftware.contains(stake) || duelist.min_stake_value <= 0 {
                    return Err(format!(
                        "content validation: duelist '{}' in '{id}' has an invalid stake",
                        duelist.id
                    ));
                }
            }
            for unit in &duelist.party {
                factories::validate_unit_grafts(
                    data,
                    &format!("duelist '{}'", duelist.id),
                    &unit.species,
                    &unit.grafts,
                )?;
            }
        }
    }
    Ok(())
}
