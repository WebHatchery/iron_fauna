//! Game-facing battle service.
//!
//! Screens provide presentation state and input intents. This boundary keeps
//! simulation advancement and command execution in the game/domain layer so
//! callers do not need to know which combat engine methods implement them.

use crate::combat::engine::Battle;
use crate::combat::PlayerCommand;
use crate::data::GameData;

pub(crate) fn advance(battle: &mut Battle, data: &GameData, dt: f32) {
    battle.update(data, dt);
}

pub(crate) fn issue(battle: &mut Battle, data: &GameData, command: PlayerCommand) -> bool {
    battle.command(data, command)
}
