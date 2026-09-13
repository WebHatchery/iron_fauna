//! Domain operations for overworld traversal.
//!
//! The overworld screen owns camera and dialogue presentation state. This
//! service owns grid movement, map interactions, encounter rolls, warps, and
//! relapse mutations so those rules can be exercised without drawing a frame.

use crate::combat::unit::UnitSpec;
use crate::combat::{Side, Stance};
use crate::data::world::{DialogueRule, DoorTarget, MapDef, MapKind, TileKind};
use crate::data::GameData;
use crate::model::story;
use crate::model::warunit::war_unit_grafts;
use crate::model::worldstate::{RegionMood, WorldState};
use crate::state::GameSession;
use crate::util::Rng;

#[derive(Debug, Clone)]
pub(crate) struct DialogueOffer {
    pub name: String,
    pub lines: Vec<String>,
    pub on_close: Option<DialogueRule>,
}

#[derive(Debug)]
pub(crate) enum TraversalEvent {
    Continue,
    OpenSettlement(DoorTarget),
    StartEncounter(Vec<UnitSpec>),
    HeartInteract(String),
}

#[derive(Debug)]
pub(crate) struct TraversalOutcome {
    pub event: TraversalEvent,
    pub dialogue: Option<DialogueOffer>,
}

impl TraversalOutcome {
    fn continue_with(dialogue: Option<DialogueOffer>) -> Self {
        Self {
            event: TraversalEvent::Continue,
            dialogue,
        }
    }
}

pub(crate) struct TraversalService {
    rng: Rng,
}

impl TraversalService {
    pub(crate) fn new(session: &GameSession) -> Self {
        Self {
            rng: Rng::new(
                0x9E37_79B9_7F4A_7C15
                    ^ (session.steps.wrapping_mul(0x2545_F491_4F6C_DD1D))
                    ^ session.battles_fought as u64,
            ),
        }
    }

    pub(crate) fn interact(
        &mut self,
        data: &GameData,
        session: &mut GameSession,
        facing: (i32, i32),
    ) -> TraversalOutcome {
        let Some(map) = data.world.map(&session.location.map_id) else {
            return TraversalOutcome::continue_with(None);
        };
        let fx = session.location.x + facing.0;
        let fy = session.location.y + facing.1;
        if let Some(npc) = map.npc_at(fx, fy) {
            let dialogue = story::select_dialogue(npc, session).map(|selection| DialogueOffer {
                name: npc.name.clone(),
                lines: selection.lines.to_vec(),
                on_close: selection.rule.cloned(),
            });
            return TraversalOutcome::continue_with(dialogue);
        }
        match map.tile(fx, fy) {
            TileKind::GestariumDoor => {
                if let Some(warp) = map.warp_at(fx, fy) {
                    session.location.map_id = warp.to_map.clone();
                    session.location.x = warp.to_x;
                    session.location.y = warp.to_y;
                    TraversalOutcome::continue_with(None)
                } else {
                    TraversalOutcome::continue_with(Some(DialogueOffer {
                        name: "Sealed Doors".to_owned(),
                        lines: vec![
                            "Warm air breathes through the seam. Deep in the hum of the earth, something is still growing.".to_owned(),
                            "The doors do not answer. Not yet.".to_owned(),
                        ],
                        on_close: None,
                    }))
                }
            }
            TileKind::Heart => map
                .factory_id
                .as_ref()
                .map(|id| TraversalOutcome {
                    event: TraversalEvent::HeartInteract(id.clone()),
                    dialogue: None,
                })
                .unwrap_or_else(|| TraversalOutcome::continue_with(None)),
            _ => TraversalOutcome::continue_with(None),
        }
    }

    pub(crate) fn step(
        &mut self,
        data: &GameData,
        session: &mut GameSession,
        direction: (i32, i32),
    ) -> TraversalOutcome {
        let Some(map) = data.world.map(&session.location.map_id) else {
            return TraversalOutcome::continue_with(None);
        };
        let nx = session.location.x + direction.0;
        let ny = session.location.y + direction.1;
        let cut_corner = direction.0 != 0
            && direction.1 != 0
            && !map.walkable(session.location.x + direction.0, session.location.y)
            && !map.walkable(session.location.x, session.location.y + direction.1);
        if !map.walkable(nx, ny) || cut_corner {
            return TraversalOutcome::continue_with(None);
        }

        session.location.x = nx;
        session.location.y = ny;
        session.steps += 1;
        let dialogue = session
            .world_state
            .tick_relapse(
                data.balance.world.relapse_per_step,
                data.balance.world.relapse_invested_mult,
            )
            .map(|factory_id| {
                let name = data
                    .factories
                    .get(&factory_id)
                    .map(|f| f.name.clone())
                    .unwrap_or(factory_id);
                DialogueOffer {
                    name: "Word on the road".to_owned(),
                    lines: vec![
                        format!("Travellers say the land around {} is wrong again. Grafted shapes drilling in the fields you brought back to life.", name),
                        "You remember planting that seed. Someone is watering it with the old poison.".to_owned(),
                        "The heart will have a new keeper. Go and meet what you made possible.".to_owned(),
                    ],
                    on_close: None,
                }
            });

        if let Some(warp) = map.warp_at(nx, ny) {
            session.location.map_id = warp.to_map.clone();
            session.location.x = warp.to_x;
            session.location.y = warp.to_y;
            return TraversalOutcome::continue_with(dialogue);
        }
        if map.tile(nx, ny) == TileKind::SettlementDoor {
            let target = map
                .door_at(nx, ny)
                .map(|d| d.target)
                .unwrap_or(DoorTarget::Hub);
            return TraversalOutcome {
                event: TraversalEvent::OpenSettlement(target),
                dialogue,
            };
        }
        if map.tile(nx, ny).encounter_prone() {
            let rate = effective_encounter_rate(map, data, session);
            if self.rng.chance(rate) {
                let mood = session.world_state.region_mood(data, &map.region);
                let armed = map.kind == MapKind::Factory || mood == RegionMood::Relapsed;
                if let Some(pack) = roll_encounter(map, data, &mut self.rng, armed) {
                    return TraversalOutcome {
                        event: TraversalEvent::StartEncounter(pack),
                        dialogue,
                    };
                }
            }
        }
        TraversalOutcome::continue_with(dialogue)
    }

    pub(crate) fn finish_dialogue(
        &self,
        data: &GameData,
        session: &mut GameSession,
        rule: Option<DialogueRule>,
    ) -> Vec<String> {
        rule.map(|rule| story::apply_dialogue_effects(&rule, session, data))
            .unwrap_or_default()
    }
}

fn effective_encounter_rate(map: &MapDef, data: &GameData, session: &GameSession) -> f32 {
    match map.kind {
        MapKind::Factory => {
            let active = map
                .factory_id
                .as_deref()
                .map(|id| session.world_state.factory_active(id))
                .unwrap_or(true);
            if active {
                map.encounter_rate
            } else {
                0.0
            }
        }
        MapKind::Overworld => {
            let mood = session.world_state.region_mood(data, &map.region);
            map.encounter_rate * WorldState::encounter_rate_mult(mood)
        }
    }
}

fn roll_encounter(
    map: &MapDef,
    data: &GameData,
    rng: &mut Rng,
    armed: bool,
) -> Option<Vec<UnitSpec>> {
    let total: u32 = map.encounters.iter().map(|entry| entry.weight).sum();
    if total == 0 {
        return None;
    }
    let mut roll = (rng.next_u64() % total as u64) as u32;
    let entry = map.encounters.iter().find(|entry| {
        if roll < entry.weight {
            true
        } else {
            roll -= entry.weight;
            false
        }
    })?;
    let count = entry.min + (rng.below((entry.max - entry.min + 1) as usize) as u32);
    let species = data.species.get(&entry.species)?;
    let tier = data.world.region(&map.region).map(|r| r.tier).unwrap_or(1);
    Some(
        (0..count)
            .map(|i| UnitSpec {
                species_id: entry.species.clone(),
                name: if count > 1 {
                    format!(
                        "{} {} {}",
                        if armed { "war-unit" } else { "wild" },
                        species.name,
                        i + 1
                    )
                } else {
                    format!(
                        "{} {}",
                        if armed { "war-unit" } else { "wild" },
                        species.name
                    )
                },
                side: Side::Enemy,
                creature_id: None,
                bond: 0.0,
                stance: Stance::Aggressive,
                grafts: if armed {
                    war_unit_grafts(species, data, tier, rng)
                } else {
                    Vec::new()
                },
            })
            .collect(),
    )
}
