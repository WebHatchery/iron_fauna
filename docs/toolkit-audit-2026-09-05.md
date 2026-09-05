# Toolkit audit — 5 September 2026

The original review had no named migration finding. Current source inspection
confirms the following shared infrastructure:

- `src/data.rs` uses toolkit embedded loading and DataRegistry for all content.
  This audit labels texture-manifest errors and adds catalogue context to
  registry errors, preserving the existing duplicate-ID rejection policy.
  Region-pack merging and factory-floor conversion remain game-specific.
- `src/game.rs` uses AssetManager, notifications, versioned save slots and the
  toolkit migration callback. `src/state.rs` owns save schema decoding.
- `src/util.rs` re-exports SeededRng; battle and world logic use it. Stable
  creature/tile hashes are coordinate/art identity functions, not RNG copies.
- `src/audio.rs` already uses SoundManager for loading and trimmed playback.
- UI screens use VirtualUi, shared text layout, panels and controls. The local
  skin cache and nine-slice drawing implement the authored sprite skin; the
  toolkit currently has no matching nine-slice primitive. Overworld scrolling
  is a clamped tile origin tied to player movement, without free-camera input.

Validation: 48 checks, formatting, strict all-target/all-feature Clippy and
Rust source-size limits passed. The final default `publish.ps1` passed Windows
and WebGL release builds, packaging with 30 assets, Preview deployment and
Project Roost tracking.
