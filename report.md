# Dotengine Report

## Overview
Dotengine is a Rust CLI that generates Hyprland desktop configurations with AI, applies them, and self-heals on reload errors. It follows a clean/hexagonal architecture and uses rule documents in the repo root to enforce visual and technical constraints.

## Repository Snapshot
- Language/runtime: Rust 2021, async with Tokio.
- Entry point: `src/main.rs` (CLI args, provider selection, rule loading, workflow execution).
- Tests: `tests/integration_tests.rs` covering workflow success, healing, and path safety.

## Architecture
- Application layer: generation + healing workflows in `src/application/`.
- Domain layer: configuration models, prompt/image payloads, templates in `src/domain/`.
- Ports: traits for AI service and system manager in `src/ports/`.
- Infrastructure: OpenAI/Gemini clients, system integration, credentials in `src/infrastructure/`.
- UI: terminal output + activity indicator in `src/ui.rs`.

## How It Works (High Level)
1. Collect prompt/template + optional image.
2. Load design rules (`design-skill.md`) and technical syntax rules (`skill.md`).
3. Generate config files via AI service.
4. Confirm, write configs with backups, and reload Hyprland.
5. If reload fails, run self-healing with diagnostics until resolved.

## Rules for App Changes
These rules are enforced in code or embedded into AI generation guidelines:

### Safe File Scope
- Only write under `.config/` and only these roots: `hypr`, `waybar`, `rofi`, `dunst`, `ags`, `quickshell`, `quick-shell`, `hyprlock`, `swaylock`, `waylock`, `swaync`.
- Reject path traversal and symlink targets.
- Back up existing files to `~/.local/share/dotengine/backups/...` before overwriting.
- Require explicit user confirmation before applying changes.

### Hyprland Syntax (from `skill.md`)
- `windowrulev2` is deprecated; use `windowrule` only.
- Match clauses must be prefixed with `match:`.
- Boolean flags must be explicit integers (`1` or `0`).
- Blur and shadow must live in nested `decoration { blur { ... } shadow { ... } }` blocks.
- Use `togglefloating` for floating toggle and `killactive` to close active windows.

### Design & Stack Guidance (from `design-skill.md`)
- Use curated palettes (Catppuccin, Nord, Tokyo Night).
- Recommend compatible component stacks based on visual archetype.
- Enforce spacing, rounding, animation guidelines for Hyprland and UI shells.

### Smart Editing Rules
- If existing configs are detected, preserve structure and only change requested parts.
- Avoid destructive full rewrites unless explicitly asked for a fresh redesign.

## AI Providers
- OpenAI: `gpt-4o` with strict JSON output format.
- Gemini: `gemini-3.5-flash` with response schema enforcement.

## Suggested Verification
- `cargo test`
