//! Overworld traversal: connected tile maps, grid movement, wild encounters,
//! NPC dialogue, and doors into settlements and Gestaria (`game_design.md` §8).

use crate::combat::unit::UnitSpec;
use crate::data::world::{DialogueRule, DoorTarget, MapDef, MapKind};
use crate::data::GameData;
use crate::game::overworld::{TraversalEvent, TraversalOutcome, TraversalService};
use crate::model::worldstate::RegionMood;
use crate::state::GameSession;
use crate::ui::{
    logical_mouse_position, logical_mouse_released, menu_button, LOGICAL_HEIGHT, LOGICAL_WIDTH,
};
use macroquad::prelude::*;
use macroquad_toolkit::assets::AssetManager;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

mod tiles;
use tiles::draw_tile;

const TILE: f32 = 40.0;

const MOVE_UP: Rect = Rect::new(1050.0, 580.0, 48.0, 34.0);
const MOVE_LEFT: Rect = Rect::new(996.0, 618.0, 48.0, 34.0);
const MOVE_DOWN: Rect = Rect::new(1050.0, 618.0, 48.0, 34.0);
const MOVE_RIGHT: Rect = Rect::new(1104.0, 618.0, 48.0, 34.0);
const INTERACT: Rect = Rect::new(1164.0, 580.0, 96.0, 34.0);
const OPEN_MENU: Rect = Rect::new(1036.0, 18.0, 96.0, 32.0);
const OPEN_CODEX: Rect = Rect::new(1140.0, 18.0, 112.0, 32.0);
const CONTINUE_DIALOGUE: Rect = Rect::new(1060.0, 584.0, 160.0, 32.0);

pub enum OverworldResult {
    Continue,
    /// Stepped through a settlement door — open the facility it leads to.
    OpenSettlement(DoorTarget),
    /// Walked into a wild pack (or a factory patrol).
    StartEncounter(Vec<UnitSpec>),
    /// Interacted with a factory heart.
    HeartInteract(String),
    /// Pressed Tab — open the codex overview.
    OpenCodex,
    BackToMenu,
}

struct DialogueBox {
    name: String,
    lines: Vec<String>,
    index: usize,
    /// Effects applied when the dialogue closes.
    on_close: Option<DialogueRule>,
}

impl DialogueBox {
    fn plain(name: &str, lines: Vec<String>) -> Self {
        Self {
            name: name.to_owned(),
            lines,
            index: 0,
            on_close: None,
        }
    }
}

pub struct OverworldScreen {
    move_timer: f32,
    facing: (i32, i32),
    /// Drawn position in tile units — glides toward the logical (grid) tile so
    /// movement and camera read as continuous while the sim stays grid-locked.
    visual_x: f32,
    visual_y: f32,
    dialogue: Option<DialogueBox>,
    traversal: TraversalService,
}

impl OverworldScreen {
    pub fn new(session: &GameSession) -> Self {
        Self {
            move_timer: 0.0,
            facing: (0, 1),
            visual_x: session.location.x as f32,
            visual_y: session.location.y as f32,
            dialogue: None,
            traversal: TraversalService::new(session),
        }
    }

    pub fn update(
        &mut self,
        data: &GameData,
        session: &mut GameSession,
        dt: f32,
    ) -> OverworldResult {
        if (is_key_pressed(KeyCode::Escape) || logical_mouse_released(OPEN_MENU))
            && self.dialogue.is_none()
        {
            return OverworldResult::BackToMenu;
        }
        if (is_key_pressed(KeyCode::Tab) || logical_mouse_released(OPEN_CODEX))
            && self.dialogue.is_none()
        {
            return OverworldResult::OpenCodex;
        }

        let Some(_map) = data.world.map(&session.location.map_id) else {
            return OverworldResult::BackToMenu;
        };

        // Glide the drawn position toward the logical tile every frame. Speed is
        // one tile per step interval, so the sprite arrives exactly as the next
        // grid step unlocks — holding a direction reads as one smooth walk.
        let glide = dt / step_time(data, session);
        self.visual_x = approach(self.visual_x, session.location.x as f32, glide);
        self.visual_y = approach(self.visual_y, session.location.y as f32, glide);

        // Dialogue swallows input until dismissed.
        if let Some(dialog) = &mut self.dialogue {
            if is_key_pressed(KeyCode::Space)
                || is_key_pressed(KeyCode::Enter)
                || is_key_pressed(KeyCode::Escape)
                || logical_mouse_released(CONTINUE_DIALOGUE)
            {
                dialog.index += 1;
                if dialog.index >= dialog.lines.len() {
                    let closed = self.dialogue.take();
                    if let Some(rule) = closed.and_then(|d| d.on_close) {
                        let notes = self.traversal.finish_dialogue(data, session, Some(rule));
                        if !notes.is_empty() {
                            self.dialogue = Some(DialogueBox::plain("Received", notes));
                        }
                    }
                }
            }
            return OverworldResult::Continue;
        }

        // The traversal service owns map interaction and returns presentation
        // data or a domain event for this view to display/dispatch.
        if is_key_pressed(KeyCode::Space)
            || is_key_pressed(KeyCode::Enter)
            || logical_mouse_released(INTERACT)
        {
            let outcome = self.traversal.interact(data, session, self.facing);
            return self.apply_traversal_outcome(outcome);
        }

        self.move_timer -= dt;
        let dir = held_direction().or_else(touch_direction);
        if let Some(dir) = dir {
            self.facing = dir;
            if self.move_timer <= 0.0 {
                let before = (
                    session.location.map_id.clone(),
                    session.location.x,
                    session.location.y,
                );
                let outcome = self.traversal.step(data, session, dir);
                let moved = before
                    != (
                        session.location.map_id.clone(),
                        session.location.x,
                        session.location.y,
                    );
                if moved {
                    self.move_timer = step_time(data, session);
                } else if matches!(outcome.event, TraversalEvent::Continue) {
                    // Bumping still turns the step timer over slightly.
                    self.move_timer = 0.08;
                }
                if moved && session.location.map_id != before.0 {
                    self.snap_visual(session);
                }
                return self.apply_traversal_outcome(outcome);
            }
        }
        OverworldResult::Continue
    }

    fn apply_traversal_outcome(&mut self, outcome: TraversalOutcome) -> OverworldResult {
        if let Some(dialogue) = outcome.dialogue {
            self.dialogue = Some(DialogueBox {
                name: dialogue.name,
                lines: dialogue.lines,
                index: 0,
                on_close: dialogue.on_close,
            });
        }
        match outcome.event {
            TraversalEvent::Continue => OverworldResult::Continue,
            TraversalEvent::OpenSettlement(target) => OverworldResult::OpenSettlement(target),
            TraversalEvent::StartEncounter(pack) => OverworldResult::StartEncounter(pack),
            TraversalEvent::HeartInteract(factory_id) => OverworldResult::HeartInteract(factory_id),
        }
    }

    /// Teleports (warps, door transits) must not glide across the gap — pin the
    /// drawn position to the new tile so the next map starts settled.
    fn snap_visual(&mut self, session: &GameSession) {
        self.visual_x = session.location.x as f32;
        self.visual_y = session.location.y as f32;
    }

    pub fn draw(&self, data: &GameData, session: &GameSession, assets: &AssetManager) {
        let Some(map) = data.world.map(&session.location.map_id) else {
            return;
        };

        // Camera: center on the (gliding) player in tile units, clamped to the
        // map bounds. Kept as a float so scrolling tracks the smooth position.
        let view_w = LOGICAL_WIDTH / TILE;
        let view_h = LOGICAL_HEIGHT / TILE;
        let cam_x =
            (self.visual_x - view_w * 0.5).clamp(0.0, (map.width() as f32 - view_w).max(0.0));
        let cam_y =
            (self.visual_y - view_h * 0.5).clamp(0.0, (map.height() as f32 - view_h).max(0.0));

        let mood = match map.kind {
            MapKind::Factory => RegionMood::Threatened,
            MapKind::Overworld => session.world_state.region_mood(data, &map.region),
        };
        clear_background(if map.kind == MapKind::Factory {
            Color::new(0.04, 0.04, 0.05, 1.0)
        } else {
            Color::new(0.05, 0.06, 0.05, 1.0)
        });
        let first_tx = cam_x.floor() as i32;
        let first_ty = cam_y.floor() as i32;
        for ty in first_ty..(first_ty + view_h.ceil() as i32 + 2).min(map.height()) {
            for tx in first_tx..(first_tx + view_w.ceil() as i32 + 2).min(map.width()) {
                let px = (tx as f32 - cam_x) * TILE;
                let py = (ty as f32 - cam_y) * TILE;
                draw_tile(
                    assets,
                    map.tile(tx, ty),
                    vec2(px, py),
                    (tx, ty),
                    map.kind,
                    mood,
                );
            }
        }

        // Building signs: label each door so the town reads as distinct shops.
        for door in &map.doors {
            let Some(label) = &door.label else { continue };
            let px = (door.x as f32 - cam_x) * TILE + TILE * 0.5;
            let py = (door.y as f32 - cam_y) * TILE;
            if px < -TILE || py < -TILE || px > LOGICAL_WIDTH + TILE || py > LOGICAL_HEIGHT + TILE {
                continue;
            }
            let w = label.len() as f32 * 7.5 + 12.0;
            draw_rectangle(
                px - w * 0.5,
                py - 20.0,
                w,
                17.0,
                Color::new(0.0, 0.0, 0.0, 0.6),
            );
            draw_text_centered_in_box_ex(
                label,
                px - w * 0.5,
                py - 20.0,
                w,
                17.0,
                TextStyle::new(13.0, Color::new(0.90, 0.82, 0.62, 1.0)),
            );
        }

        for npc in &map.npcs {
            let px = (npc.x as f32 - cam_x) * TILE + TILE * 0.5;
            let py = (npc.y as f32 - cam_y) * TILE + TILE * 0.5;
            if px < -TILE || py < -TILE || px > LOGICAL_WIDTH + TILE || py > LOGICAL_HEIGHT + TILE {
                continue;
            }
            draw_circle(px, py - 6.0, 9.0, Color::new(0.80, 0.70, 0.55, 1.0));
            draw_rectangle(px - 7.0, py, 14.0, 14.0, Color::new(0.45, 0.40, 0.55, 1.0));
        }

        // Player, drawn at the smooth (gliding) position.
        let px = (self.visual_x - cam_x) * TILE + TILE * 0.5;
        let py = (self.visual_y - cam_y) * TILE + TILE * 0.5;
        draw_circle(px, py - 7.0, 9.0, Color::new(0.92, 0.85, 0.72, 1.0));
        draw_rectangle(
            px - 7.0,
            py - 1.0,
            14.0,
            15.0,
            Color::new(0.55, 0.35, 0.28, 1.0),
        );
        // Facing tick.
        draw_circle(
            px + self.facing.0 as f32 * 10.0,
            py - 7.0 + self.facing.1 as f32 * 6.0,
            2.5,
            Color::new(0.2, 0.15, 0.1, 1.0),
        );

        self.draw_hud(data, session, map);
        if let Some(dialog) = &self.dialogue {
            draw_dialogue(dialog, &data.config.controls.dialogue);
        } else {
            self.draw_touch_controls();
        }
    }

    fn draw_touch_controls(&self) {
        let mouse = logical_mouse_position();
        menu_button(MOVE_UP, "▲", true, mouse);
        menu_button(MOVE_LEFT, "◀", true, mouse);
        menu_button(MOVE_DOWN, "▼", true, mouse);
        menu_button(MOVE_RIGHT, "▶", true, mouse);
        menu_button(INTERACT, "INTERACT", true, mouse);
        menu_button(OPEN_MENU, "MENU", true, mouse);
        menu_button(OPEN_CODEX, "CODEX", true, mouse);
    }

    fn draw_hud(&self, data: &GameData, session: &GameSession, map: &MapDef) {
        let region = data
            .world
            .region(&map.region)
            .map(|r| r.name.as_str())
            .unwrap_or("?");
        draw_rectangle(12.0, 10.0, 380.0, 34.0, Color::new(0.0, 0.0, 0.0, 0.55));
        draw_ui_text_ex(
            &format!("{} — {}", map.name, region),
            22.0,
            33.0,
            TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
        );

        // Quest tracker: the accepted objective, or a turn-in prompt.
        if let Some(line) = crate::model::quest::tracker_line(session, data) {
            let gold = Color::new(0.95, 0.82, 0.45, 1.0);
            draw_rectangle(12.0, 48.0, 380.0, 26.0, Color::new(0.0, 0.0, 0.0, 0.5));
            draw_rectangle(12.0, 48.0, 4.0, 26.0, gold);
            draw_ui_text_ex(
                &format!("Quest: {}", line),
                24.0,
                66.0,
                TextStyle::new(15.0, gold).params(),
            );
        }

        draw_rectangle(
            12.0,
            LOGICAL_HEIGHT - 40.0,
            700.0,
            30.0,
            Color::new(0.0, 0.0, 0.0, 0.45),
        );
        draw_ui_text_ex(
            &data.config.controls.overworld,
            22.0,
            LOGICAL_HEIGHT - 19.0,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
    }
}

/// Combine held keys into an 8-way step; opposite keys cancel so pressing
/// left+right (or up+down) leaves that axis neutral.
fn held_direction() -> Option<(i32, i32)> {
    let mut dx = 0;
    let mut dy = 0;
    if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
        dx -= 1;
    }
    if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
        dx += 1;
    }
    if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
        dy -= 1;
    }
    if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
        dy += 1;
    }
    if dx == 0 && dy == 0 {
        None
    } else {
        Some((dx, dy))
    }
}

fn touch_direction() -> Option<(i32, i32)> {
    let mouse = logical_mouse_position();
    if !is_mouse_button_down(MouseButton::Left) {
        return None;
    }
    if MOVE_UP.contains_point(mouse) {
        Some((0, -1))
    } else if MOVE_LEFT.contains_point(mouse) {
        Some((-1, 0))
    } else if MOVE_DOWN.contains_point(mouse) {
        Some((0, 1))
    } else if MOVE_RIGHT.contains_point(mouse) {
        Some((1, 0))
    } else {
        None
    }
}

/// Fliers earn their keep as overworld pace (`creature.md` §2.4).
fn step_time(data: &GameData, session: &GameSession) -> f32 {
    let has_flier = session
        .profile
        .roster
        .party_members()
        .any(|c| c.species(data).natural_flight);
    if has_flier {
        0.10
    } else {
        0.15
    }
}

/// Encounter richness responds to the region's verdict; dormant factories
/// stop birthing patrols entirely.
fn draw_dialogue(dialog: &DialogueBox, control_hint: &str) {
    let rect = Rect::new(60.0, LOGICAL_HEIGHT - 170.0, LOGICAL_WIDTH - 120.0, 130.0);
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.07, 0.08, 0.10, 0.97))
            .with_border(1.5, Color::new(0.5, 0.55, 0.65, 0.8)),
    );
    draw_ui_text_ex(
        &dialog.name,
        rect.x + 20.0,
        rect.y + 30.0,
        TextStyle::new(18.0, Color::new(0.85, 0.78, 0.6, 1.0)).params(),
    );
    if let Some(line) = dialog.lines.get(dialog.index) {
        draw_text_block(
            line,
            rect.x + 20.0,
            rect.y + 46.0,
            rect.w - 40.0,
            rect.h - 60.0,
            17.0,
            5.0,
            dark::TEXT,
        );
    }
    draw_ui_text_ex(
        control_hint,
        rect.right() - 260.0,
        rect.bottom() - 14.0,
        TextStyle::new(14.0, dark::TEXT_DIM).params(),
    );
    menu_button(
        CONTINUE_DIALOGUE,
        "CONTINUE",
        true,
        logical_mouse_position(),
    );
}
