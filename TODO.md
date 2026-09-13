# TODO — IRON FAUNA

## Standards compliance

- [ ] Expose testable game logic through `src/lib.rs`, have `src/main.rs` use it, and migrate all tests and test-only helpers from `src/` into `tests/` before expanding coverage. Preserve regressions and review case counts by feature against the five-case target. Correct the obsolete “non-test lines” comment in `tests/code_standards.rs` (§2.2, §11).
- [ ] Add visible touch controls for overworld movement, interaction, dialogue advance, and menu/Codex access in `src/ui/overworld.rs`; add battle command selection, targeting, pause/resume, forced hop, and outcome continuation in `src/ui/battle/`. Provide touch-accessible return/navigation controls wherever shortcuts are the only route, including closing the Codex. Verify a keyboard-free gameplay loop at multiple browser sizes (§7.5).
- [ ] Update in-game help/tutorial prompts, `README.md`, and `game_page.json` to name the exact visible controls and their optional shortcuts. Replace outdated battle hotkeys and the incorrect Tab “Rider hop targeting” description with the implemented command-menu controls (§7.5).
- [ ] Move traversal, encounters, dialogue effects, and relapse mutations out of `OverworldScreen`; move battle ownership, simulation updates, and command execution out of `BattleScreen` and its menu handlers. Keep UI presentation state local and dispatch gameplay intents through `Game` and domain services (§5.1, §7.1–7.2).
- [ ] Extract cohesive helpers from functions exceeding 100 lines, including `OutfitScreen::draw_creature_panel`, `OverworldScreen::update`, and `Battle::try_attack`. Split growing modules by responsibility as needed, using named module files and preserving the 800-total-line gate (§2.2–2.3, §4.1).
- [ ] Add startup semantic validation to `GameData::load`: reject duplicate IDs before registry/region merges, invalid references and map coordinates, and invalid balance values. Reuse the rules currently asserted only in `src/data/**/tests.rs`, return actionable content diagnostics, and add malformed-data regression cases through the library API (§5.3, §6).
- [ ] Move the starter identity, inventory, equipment, and other hardcoded gameplay configuration in `src/state.rs`, plus player-facing strings in `src/ui/`, `src/model/`, and `src/game/`, into typed JSON under `assets/` loaded through the toolkit (§5.3).
- [ ] Report asset-pack and sound loading failures in `src/game.rs` and `src/audio.rs`, detect incomplete texture loads, and handle rejected starter equipment in `src/state.rs`; retain usable fallbacks with clear diagnostics (§6, §8.3).
- [ ] Use toolkit button interaction helpers for `src/ui.rs::menu_button` and `src/ui/skin.rs::button`, retaining the existing visual skin and release activation (§7.4).
- [ ] Remove unused `data`/RNG parameters hidden by `let _ = ...` or `_rng` in `src/ui/bestiary.rs` and `src/ui/creature_art.rs`; replace the undocumented argument-count suppression on `OutfitScreen::roster_row` with a cohesive context or a justified comment. Remove the unreferenced template content in `assets/data/actions.json` (§1.4, §4.3, §10.2).

## Content and balance

- [ ] Author bounties for the five outer regions and wire their NPC offers, tracking, and rewards; `assets/data/quests.json` currently contains only `morning_thinning`. Extend objective kinds where the authored quests require them.
- [ ] Run a deterministic encounter/loadout balance pass across the species roster and regional encounters; tune Power Budget/trait costs, limb mounts, graftware Power Draw, and per-size party slot costs in the relevant JSON. Record the resulting rationale in the design documents and preserve representative regression coverage (`creature.md` §9; `combat.md` §2.1, §9).
