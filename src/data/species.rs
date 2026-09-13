//! Species (chassis) definitions — see `creature.md`.
//!
//! A species is a chassis: Power, Size, Speed, Limb Count, Element. All
//! combat-facing numbers are derived from these via `BalanceConfig` curves so
//! balance lives in `assets/data/balance.json`, not in code.

use crate::data::balance::BalanceConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Element {
    BioElectric,
    Plant,
    Rock,
    Fire,
    Water,
    Poison,
}

impl Element {
    pub fn display_name(self) -> &'static str {
        match self {
            Element::BioElectric => "Bio-Electric",
            Element::Plant => "Plant",
            Element::Rock => "Rock",
            Element::Fire => "Fire",
            Element::Water => "Water",
            Element::Poison => "Poison",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SizeClass {
    Small,
    Medium,
    Large,
    Huge,
}

impl SizeClass {
    /// Party/battle field budget cost (`combat.md` §2.1).
    pub fn slot_cost(self, balance: &BalanceConfig) -> u32 {
        balance.party_slot_cost.get(self)
    }

    pub fn display_name(self) -> &'static str {
        match self {
            SizeClass::Small => "Small",
            SizeClass::Medium => "Medium",
            SizeClass::Large => "Large",
            SizeClass::Huge => "Huge",
        }
    }
}

/// Strain-tolerance personality (`game_design.md` §4.3): gentler creatures
/// have the lowest strain tolerance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Temperament {
    Gentle,
    Placid,
    Wary,
    Fierce,
}

impl Temperament {
    pub fn display_name(self) -> &'static str {
        match self {
            Temperament::Gentle => "Gentle",
            Temperament::Placid => "Placid",
            Temperament::Wary => "Wary",
            Temperament::Fierce => "Fierce",
        }
    }
}

/// Limb-count archetype (`creature.md` §2.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LimbArchetype {
    Flier,
    Standard,
    Utility,
    Heavy,
}

impl LimbArchetype {
    pub fn display_name(self) -> &'static str {
        match self {
            LimbArchetype::Flier => "Flier",
            LimbArchetype::Standard => "Standard",
            LimbArchetype::Utility => "Utility",
            LimbArchetype::Heavy => "Heavy",
        }
    }
}

/// Graftware weight class a mount point accepts (also used by graftware defs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum WeightClass {
    Light,
    Medium,
    Heavy,
}

impl WeightClass {
    pub fn display_name(self) -> &'static str {
        match self {
            WeightClass::Light => "Light",
            WeightClass::Medium => "Medium",
            WeightClass::Heavy => "Heavy",
        }
    }
}

/// Where a limb sits on the sprite — drives called-shot directional mapping
/// (`combat.md` §3.1: up = head/back, down = legs, left/right = arms).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LimbRegion {
    Head,
    Back,
    ArmLeft,
    ArmRight,
    Legs,
    Tail,
}

impl LimbRegion {
    pub fn display_name(self) -> &'static str {
        match self {
            LimbRegion::Head => "Head",
            LimbRegion::Back => "Back",
            LimbRegion::ArmLeft => "Left Arm",
            LimbRegion::ArmRight => "Right Arm",
            LimbRegion::Legs => "Legs",
            LimbRegion::Tail => "Tail",
        }
    }
}

/// One war-body limb: a targetable body segment that can host graftware
/// mounts. Limbs regrow while the core lives; mounted graftware does not.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimbDef {
    pub id: String,
    pub name: String,
    pub region: LimbRegion,
    /// Mount points on this limb (0..=2), by accepted weight class.
    #[serde(default)]
    pub mounts: Vec<WeightClass>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub element: Element,
    pub size: SizeClass,
    pub temperament: Temperament,
    pub archetype: LimbArchetype,
    /// Biological capacity, 0-100 (`creature.md` §2.1).
    pub power: u32,
    /// 0-100. Fast = dodge + battle movement + overworld pace.
    pub speed: u32,
    #[serde(default)]
    pub natural_flight: bool,
    /// Flat innate shell armor (subtracted from limb hits).
    #[serde(default)]
    pub innate_armor: u32,
    pub limbs: Vec<LimbDef>,
    /// Content tier 1-4: gates wild placement and factory depth.
    pub tier: u32,
    /// Biome tags for encounter tables (e.g. "meadow", "marsh", "caldera").
    #[serde(default)]
    pub biomes: Vec<String>,
}

/// Battle-facing numbers derived from chassis stats through balance curves.
#[derive(Debug, Clone, Copy)]
pub struct DerivedStats {
    pub power_capacity: f32,
    pub vigor_max: f32,
    pub vigor_regen: f32,
    pub strain_threshold: f32,
    pub limb_hp: f32,
    pub core_hp: f32,
    pub regrow_hp_per_sec: f32,
    /// 0-1 chance to fully evade an incoming hit.
    pub dodge: f32,
    /// Additive accuracy bonus for slow, stable chassis.
    pub accuracy_bonus: f32,
}

impl SpeciesDef {
    pub fn power_budget_cost(&self, bal: &BalanceConfig) -> f32 {
        let extra_limbs = self.limbs.len().saturating_sub(4) as f32;
        self.power as f32 * bal.power_budget.power_per_point
            + self.speed as f32 * bal.power_budget.speed_per_point
            + extra_limbs * bal.power_budget.extra_limb_cost
            + if self.natural_flight {
                bal.power_budget.natural_flight_cost
            } else {
                0.0
            }
            + self.innate_armor as f32 * bal.power_budget.innate_armor_per_point
            + bal.power_budget.size_cost.get(self.size)
    }

    pub fn derived(&self, bal: &BalanceConfig) -> DerivedStats {
        let c = &bal.curves;
        let power = self.power as f32;
        let speed = self.speed as f32;
        let size_i = self.size;
        DerivedStats {
            power_capacity: power * c.power_capacity_mult,
            vigor_max: c.vigor_base + power * c.vigor_per_power + c.vigor_size_bonus.get(size_i),
            vigor_regen: c.vigor_regen_base + power * c.vigor_regen_per_power,
            strain_threshold: (c.strain_base + power * c.strain_per_power)
                * c.temperament_strain_mult.get(self.temperament),
            limb_hp: (c.limb_hp_base + power * c.limb_hp_per_power)
                * c.limb_hp_size_mult.get(size_i),
            core_hp: c.core_hp_base + power * c.core_hp_per_power,
            regrow_hp_per_sec: c.regrow_base + power * c.regrow_per_power,
            dodge: (speed * c.dodge_per_speed + c.size_dodge_bonus.get(size_i)).clamp(0.0, 0.2),
            accuracy_bonus: ((c.accuracy_slow_pivot - speed).max(0.0)) * c.accuracy_slow_per_point,
        }
    }

    pub fn mount_count(&self) -> usize {
        self.limbs.iter().map(|l| l.mounts.len()).sum()
    }

    pub fn limb(&self, limb_id: &str) -> Option<&LimbDef> {
        self.limbs.iter().find(|l| l.id == limb_id)
    }
}
