//! Player commands and attack/effect resolution.

use crate::combat::engine::Battle;
use crate::combat::events::BattleEvent;
use crate::combat::unit::Dot;
use crate::combat::{CalledTarget, PlayerCommand, Side, UnitId, WeaponRef};
use crate::data::graftware::{BoostEffect, GraftEffect};
use crate::data::item::ConsumableEffect;
use crate::data::GameData;

struct AttackPlan {
    damage: f32,
    vigor_cost: f32,
    cooldown: f32,
    synergy: f32,
    draw: f32,
    boost_active: bool,
}

impl Battle {
    /// Issues a player command. Returns false if the command was invalid
    /// (wrong state, on cooldown, out of vigor/range).
    pub fn command(&mut self, data: &GameData, cmd: PlayerCommand) -> bool {
        if self.over() {
            return false;
        }
        match cmd {
            PlayerCommand::SetStance { unit, stance } => {
                let Some(u) = self.units.get_mut(unit) else {
                    return false;
                };
                if u.side != Side::Player || !u.alive() {
                    return false;
                }
                u.stance = stance;
                self.events.push(BattleEvent::StanceChanged { unit });
                true
            }
            PlayerCommand::BeginHop { to } => self.begin_hop(data, to),
            other => {
                let Some(id) = self.ridden_unit() else {
                    return false;
                };
                if !self.is_commandable(id) {
                    return false;
                }
                self.ridden_command(data, id, other)
            }
        }
    }

    fn ridden_command(&mut self, data: &GameData, id: UnitId, cmd: PlayerCommand) -> bool {
        match cmd {
            PlayerCommand::Fire {
                mount,
                target,
                called,
            } => {
                let ok = self.try_attack(data, id, target, WeaponRef::Mount(mount), called, true);
                if ok {
                    self.ridden_ready_announced = false;
                }
                ok
            }
            PlayerCommand::NaturalAttack { target, called } => {
                let ok = self.try_attack(data, id, target, WeaponRef::Natural, called, true);
                if ok {
                    self.ridden_ready_announced = false;
                }
                ok
            }
            PlayerCommand::TriggerUtility { mount, ally } => {
                self.trigger_utility(data, id, mount, ally)
            }
            PlayerCommand::Regrow { limb } => self.begin_regrow(id, limb),
            PlayerCommand::Reinforce => self.reinforce(data, id),
            PlayerCommand::SetStance { .. } | PlayerCommand::BeginHop { .. } => false,
        }
    }

    /// Rider-hop (`combat.md` §5): a real, costed action — the rider is
    /// exposed mid-transit and both creatures fall back to standing orders.
    pub(crate) fn begin_hop(&mut self, data: &GameData, to: UnitId) -> bool {
        if self.rider.hop.is_some() {
            return false;
        }
        let valid = self
            .units
            .get(to)
            .is_some_and(|u| u.side == Side::Player && u.alive());
        if !valid || self.rider.mounted_on == Some(to) {
            return false;
        }
        let from = self.rider.mounted_on;
        let time = data.balance.battle.hop_transit_time * self.rider_mods.hop_time_mult;
        self.rider.mounted_on = None;
        self.rider.hop = Some((to, time));
        self.events.push(BattleEvent::HopStarted { from, to });
        true
    }

    pub(crate) fn begin_regrow(&mut self, id: UnitId, limb: usize) -> bool {
        let u = &mut self.units[id];
        let Some(l) = u.limbs.get(limb) else {
            return false;
        };
        if !l.severed {
            return false;
        }
        u.regrow_target = Some(limb);
        true
    }

    pub(crate) fn reinforce(&mut self, data: &GameData, id: UnitId) -> bool {
        let bal = &data.balance;
        let ridden = self.ridden_unit() == Some(id);
        let mult = if ridden {
            self.rider_mods.reinforce_mult
        } else {
            1.0
        };
        let u = &mut self.units[id];
        if u.reinforce_cooldown > 0.0 || u.vigor < bal.vigor.reinforce_cost {
            return false;
        }
        u.vigor -= bal.vigor.reinforce_cost;
        u.reinforce_cooldown = bal.vigor.reinforce_cooldown;
        let cap = u.core_max * bal.vigor.reinforce_shield_cap_frac;
        let amount = bal.vigor.reinforce_shield * mult;
        u.shield = (u.shield + amount).min(cap);
        self.events.push(BattleEvent::Shielded { unit: id, amount });
        true
    }

    pub(crate) fn trigger_utility(
        &mut self,
        data: &GameData,
        id: UnitId,
        mount: usize,
        ally: Option<UnitId>,
    ) -> bool {
        let ridden = self.ridden_unit() == Some(id);
        let (effect, synergy) = {
            let u = &self.units[id];
            let Some(m) = u.mounts.get(mount) else {
                return false;
            };
            if !m.usable() || !u.limbs[m.limb_index].intact() || m.cooldown > 0.0 {
                return false;
            }
            let Some(def) = data.graftware.get(&m.def_id) else {
                return false;
            };
            let Some(effect) = def.effect else {
                return false;
            };
            (effect, u.synergy(data, def))
        };
        let amplify = if ridden {
            self.ridden_boosts(data, id)
                .filter_map(|b| match b {
                    BoostEffect::Amplify { mult } => Some(mult),
                    _ => None,
                })
                .product::<f32>()
                .max(1.0)
        } else {
            1.0
        };

        match effect {
            GraftEffect::Heal {
                amount,
                cooldown,
                vigor_cost,
            } => {
                if self.units[id].vigor < vigor_cost {
                    return false;
                }
                let target = ally
                    .filter(|&a| {
                        self.units
                            .get(a)
                            .is_some_and(|u| u.side == Side::Player && u.alive())
                    })
                    .unwrap_or(id);
                self.units[id].vigor -= vigor_cost;
                self.units[id].mounts[mount].cooldown = cooldown;
                let healed = amount * synergy * amplify;
                self.heal_unit(target, healed);
                self.events.push(BattleEvent::Healed {
                    source: id,
                    target,
                    amount: healed,
                });
                true
            }
            GraftEffect::ShieldCore {
                amount,
                cooldown,
                vigor_cost,
            } => {
                let bal = &data.balance;
                let u = &mut self.units[id];
                if u.vigor < vigor_cost {
                    return false;
                }
                u.vigor -= vigor_cost;
                u.mounts[mount].cooldown = cooldown;
                let cap = u.core_max * bal.vigor.reinforce_shield_cap_frac;
                let gained = amount * synergy * amplify;
                u.shield = (u.shield + gained).min(cap);
                self.events.push(BattleEvent::Shielded {
                    unit: id,
                    amount: gained,
                });
                true
            }
            // Passive effects have no trigger.
            _ => false,
        }
    }

    pub(crate) fn heal_unit(&mut self, id: UnitId, amount: f32) {
        let u = &mut self.units[id];
        // Heal the most-wounded intact limb; overflow soothes the core.
        let target = u
            .limbs
            .iter_mut()
            .filter(|l| l.intact() && l.hp < l.max_hp)
            .min_by(|a, b| {
                (a.hp / a.max_hp)
                    .partial_cmp(&(b.hp / b.max_hp))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        if let Some(limb) = target {
            limb.hp = (limb.hp + amount).min(limb.max_hp);
        } else {
            u.core_hp = (u.core_hp + amount * 0.5).min(u.core_max);
        }
    }

    /// The damage multiplier and optional burn from the ammo loaded in a
    /// weapon (`(1.0, None)` for natural attacks or standard fire).
    fn ammo_effect(
        &self,
        data: &GameData,
        attacker: UnitId,
        weapon: WeaponRef,
    ) -> (f32, Option<(f32, f32)>) {
        let WeaponRef::Mount(m) = weapon else {
            return (1.0, None);
        };
        let Some(ammo) = self.units[attacker]
            .mounts
            .get(m)
            .and_then(|mt| mt.ammo.as_ref())
        else {
            return (1.0, None);
        };
        match data.items.get(&ammo.def_id).map(|d| d.effect) {
            Some(ConsumableEffect::Ammo {
                damage_mult,
                burn_dps,
                burn_secs,
                ..
            }) => (
                damage_mult,
                (burn_dps > 0.0).then_some((burn_dps, burn_secs)),
            ),
            _ => (1.0, None),
        }
    }

    /// Attempts an attack; validates range, cooldown, and vigor.
    pub(crate) fn try_attack(
        &mut self,
        data: &GameData,
        attacker: UnitId,
        target: UnitId,
        weapon: WeaponRef,
        called: Option<CalledTarget>,
        is_player_call: bool,
    ) -> bool {
        let Some(plan) = self.prepare_attack(data, attacker, target, weapon) else {
            return false;
        };
        let (ammo_mult, ammo_burn) = self.ammo_effect(data, attacker, weapon);
        self.spend_attack(data, attacker, weapon, &plan);

        // Accuracy roll.
        let bal = &data.balance;
        let mut acc =
            bal.battle.base_accuracy + self.units[attacker].effective_accuracy_bonus(data);
        if called.is_some() {
            acc *= bal.battle.called_shot_accuracy_mult;
            if is_player_call {
                acc *= self.rider_mods.called_shot_mult;
            }
        }
        let hit_chance = (acc * (1.0 - self.units[target].dodge)).clamp(0.4, 0.97);
        if !self.rng.chance(hit_chance) {
            self.events.push(BattleEvent::Miss { attacker, target });
            return true; // the shot happened; it just missed
        }

        let dealt = plan.damage * plan.synergy * ammo_mult * data.balance.battle.weapon_damage_mult;
        self.apply_damage(data, attacker, target, dealt, called);
        self.apply_ammo_burn(target, ammo_burn);
        self.apply_attack_boosts(data, attacker, target, dealt, plan.boost_active);
        true
    }

    fn prepare_attack(
        &self,
        data: &GameData,
        attacker: UnitId,
        target: UnitId,
        weapon: WeaponRef,
    ) -> Option<AttackPlan> {
        let target_unit = self.units.get(target)?;
        let attacker_unit = self.units.get(attacker)?;
        if !target_unit.alive() || attacker_unit.side == target_unit.side {
            return None;
        }
        let plan = match weapon {
            WeaponRef::Natural => {
                if attacker_unit.natural_cooldown > 0.0 {
                    return None;
                }
                AttackPlan {
                    damage: attacker_unit.natural_damage,
                    vigor_cost: 2.0,
                    cooldown: data.balance.battle.natural_attack_cooldown,
                    synergy: 1.0,
                    draw: 0.0,
                    boost_active: false,
                }
            }
            WeaponRef::Mount(m) => {
                let mount = attacker_unit.mounts.get(m)?;
                if !mount.usable()
                    || !attacker_unit.limbs[mount.limb_index].intact()
                    || mount.cooldown > 0.0
                {
                    return None;
                }
                let def = data.graftware.get(&mount.def_id)?;
                if !def.is_weapon() {
                    return None;
                }
                AttackPlan {
                    damage: def.damage,
                    vigor_cost: def.vigor_cost,
                    cooldown: def.cooldown,
                    synergy: attacker_unit.synergy(data, def),
                    draw: def.power_draw as f32,
                    boost_active: self.ridden_unit() == Some(attacker),
                }
            }
        };
        (attacker_unit.vigor >= plan.vigor_cost).then_some(plan)
    }

    fn spend_attack(
        &mut self,
        data: &GameData,
        attacker: UnitId,
        weapon: WeaponRef,
        plan: &AttackPlan,
    ) {
        let u = &mut self.units[attacker];
        u.vigor -= plan.vigor_cost;
        match weapon {
            WeaponRef::Natural => u.natural_cooldown = plan.cooldown,
            WeaponRef::Mount(m) => u.mounts[m].cooldown = plan.cooldown,
        }
        u.strain += plan.draw * data.balance.strain.fire_gain_per_draw;
        if let WeaponRef::Mount(m) = weapon {
            if let Some(ammo) = &mut u.mounts[m].ammo {
                ammo.rounds = ammo.rounds.saturating_sub(1);
                if ammo.rounds == 0 {
                    u.mounts[m].ammo = None;
                }
            }
        }
    }

    fn apply_ammo_burn(&mut self, target: UnitId, ammo_burn: Option<(f32, f32)>) {
        let Some((dps, secs)) = ammo_burn else {
            return;
        };
        if !self.units[target].alive() {
            return;
        }
        let dot = Dot {
            dps,
            remaining: secs,
        };
        let intact = self.units[target].intact_limbs();
        if intact.is_empty() {
            self.units[target].core_dots.push(dot);
        } else {
            let li = intact[self.rng.below(intact.len())];
            self.units[target].limb_dots.push((li, dot));
        }
    }

    fn apply_attack_boosts(
        &mut self,
        data: &GameData,
        attacker: UnitId,
        target: UnitId,
        dealt: f32,
        boost_active: bool,
    ) {
        if !boost_active {
            return;
        }
        let boosts: Vec<BoostEffect> = self.ridden_boosts(data, attacker).collect();
        for boost in boosts {
            match boost {
                BoostEffect::Corrode { dps, duration } => self.units[target].core_dots.push(Dot {
                    dps,
                    remaining: duration,
                }),
                BoostEffect::ChainArc {
                    extra_targets,
                    falloff,
                } => {
                    let mut chained = 0;
                    for other in self.alive_on(self.units[target].side) {
                        if other != target && chained < extra_targets {
                            chained += 1;
                            self.apply_damage(data, attacker, other, dealt * falloff, None);
                        }
                    }
                }
                BoostEffect::Barrage { extra_shots } => {
                    for _ in 0..extra_shots {
                        if self.units[target].alive() {
                            self.apply_damage(data, attacker, target, dealt * 0.5, None);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Routes damage per the anatomy: called mount → graft; otherwise a limb;
    /// once all limbs are gone, the exposed core.
    pub(crate) fn apply_damage(
        &mut self,
        data: &GameData,
        attacker: UnitId,
        target: UnitId,
        amount: f32,
        called: Option<CalledTarget>,
    ) {
        if !self.units[target].alive() {
            return;
        }
        let spill_frac = data.balance.battle.graft_spill_frac;

        match called {
            Some(CalledTarget::Mount(mi))
                if self.units[target].mounts.get(mi).is_some_and(|m| {
                    m.usable() && self.units[target].limbs[m.limb_index].intact()
                }) =>
            {
                // Chip the graft itself; spill wounds the host limb.
                let (destroyed, spill, limb_index) = {
                    let m = &mut self.units[target].mounts[mi];
                    m.graft_hp -= amount;
                    let destroyed = m.graft_hp <= 0.0;
                    let spill = if destroyed {
                        (-m.graft_hp).max(0.0) * spill_frac
                    } else {
                        amount * spill_frac * 0.5
                    };
                    (destroyed, spill, m.limb_index)
                };
                self.events.push(BattleEvent::Hit {
                    attacker,
                    target,
                    amount,
                    to_core: false,
                });
                if destroyed {
                    self.destroy_mount(data, target, mi);
                }
                if spill > 0.0 {
                    self.damage_limb_raw(data, target, limb_index, spill);
                }
            }
            Some(CalledTarget::Limb(li))
                if self.units[target].limbs.get(li).is_some_and(|l| l.intact()) =>
            {
                self.events.push(BattleEvent::Hit {
                    attacker,
                    target,
                    amount,
                    to_core: false,
                });
                self.damage_limb_armored(data, target, li, amount);
            }
            _ => {
                let intact = self.units[target].intact_limbs();
                if intact.is_empty() {
                    // Core exposed: shield, then the core itself.
                    let dealt = amount * data.balance.battle.exposed_core_damage_mult;
                    self.events.push(BattleEvent::Hit {
                        attacker,
                        target,
                        amount: dealt,
                        to_core: true,
                    });
                    self.damage_core_raw(target, dealt);
                } else {
                    let li = intact[self.rng.below(intact.len())];
                    self.events.push(BattleEvent::Hit {
                        attacker,
                        target,
                        amount,
                        to_core: false,
                    });
                    self.damage_limb_armored(data, target, li, amount);
                }
            }
        }
    }

    fn damage_limb_armored(&mut self, data: &GameData, target: UnitId, li: usize, amount: f32) {
        let armor = self.units[target].limb_armor(data, li);
        // Armor can't fully negate — a fifth always bleeds through.
        let dealt = (amount - armor).max(amount * 0.2);
        self.damage_limb_raw(data, target, li, dealt);
    }

    pub(crate) fn damage_limb_raw(
        &mut self,
        data: &GameData,
        target: UnitId,
        li: usize,
        amount: f32,
    ) {
        let severed = {
            let limb = &mut self.units[target].limbs[li];
            if !limb.intact() {
                return;
            }
            limb.hp -= amount;
            limb.hp <= 0.0
        };
        if severed {
            self.sever_limb(data, target, li);
        }
    }

    /// Blowing off a limb detaches its graftware as salvage
    /// (`game_design.md` §6 — salvage economy).
    fn sever_limb(&mut self, data: &GameData, target: UnitId, li: usize) {
        {
            let limb = &mut self.units[target].limbs[li];
            limb.severed = true;
            limb.hp = 0.0;
            limb.regrow_hp = 0.0;
        }
        let limb_name = {
            let u = &self.units[target];
            u.limb_def(data, &u.limbs[li]).name.clone()
        };
        self.events.push(BattleEvent::LimbSevered {
            unit: target,
            limb_name,
        });
        self.units[target].limb_dots.retain(|(l, _)| *l != li);

        let enemy_side = self.units[target].side == Side::Enemy;
        let salvage_chance =
            data.balance.battle.salvage_drop_chance + self.rider_mods.salvage_bonus;
        let mount_ids: Vec<usize> = self.units[target]
            .mounts
            .iter()
            .enumerate()
            .filter(|(_, m)| m.limb_index == li && m.usable())
            .map(|(i, _)| i)
            .collect();
        for mi in mount_ids {
            let def_id = self.units[target].mounts[mi].def_id.clone();
            self.units[target].mounts[mi].detached = true;
            if enemy_side && self.rng.chance(salvage_chance) {
                self.salvage.push(def_id.clone());
                self.events.push(BattleEvent::SalvageDropped { def_id });
            }
        }

        if self.units[target].core_exposed() {
            self.events.push(BattleEvent::CoreExposed { unit: target });
        }
    }

    fn destroy_mount(&mut self, data: &GameData, target: UnitId, mi: usize) {
        let name = self.graft_name(data, target, mi);
        self.units[target].mounts[mi].destroyed = true;
        self.events.push(BattleEvent::GraftDestroyed {
            unit: target,
            graft_name: name,
        });
    }

    pub(crate) fn damage_core_raw(&mut self, target: UnitId, amount: f32) {
        let cracked = {
            let u = &mut self.units[target];
            let after_shield = (amount - u.shield).max(0.0);
            u.shield = (u.shield - amount).max(0.0);
            u.core_hp -= after_shield;
            u.core_hp <= 0.0
        };
        if cracked {
            self.crack_core(target);
        }
    }

    /// Cracking the core downs the creature — a capture, a freed core, or a
    /// yield; never a kill (`game_design.md` §4.1).
    fn crack_core(&mut self, target: UnitId) {
        let u = &mut self.units[target];
        u.downed = true;
        u.core_hp = 0.0;
        self.events.push(BattleEvent::CoreCracked { unit: target });

        if self.rider.mounted_on == Some(target) {
            self.rider.mounted_on = None;
            self.events.push(BattleEvent::RiderExposed);
        }
    }
}
