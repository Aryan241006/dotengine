# Hyprland Syntax Skill

This document is the config-syntax reference for Dotengine. It should help the generator choose the correct Hyprland syntax family, avoid invalid keywords, and produce a setup that reloads cleanly.

## 1. Version-Family Selection

- Always match the emitted config format to the detected Hyprland family.
- Newer Hyprland releases may use Lua-based configuration surfaces, while older releases use `hyprland.conf` and hyprlang syntax.
- When version detection is uncertain, prefer the syntax family supported by the installed release rather than mixing styles.
- Do not emit syntax from more than one family in the same file.

## 2. Config File Layout

- Keep Hyprland config in the canonical Hypr config directory.
- Use one directive per line.
- Group related blocks together:
  - general
  - monitors
  - input
  - decoration
  - animations
  - binds
  - window rules
  - layer rules
  - startup
- Treat app configs as separate files. Waybar, Rofi, SwayNC, Hyprlock, Hyprpaper, and similar tools do not belong inside the Hyprland config body.

## 3. Core Syntax Rules

- Follow the documented block structure for the detected version.
- Use explicit values for booleans and numeric settings.
- Prefer documented keys only; avoid inventing field names from UI assumptions.
- When in doubt, keep the generated config conservative and reload-safe.

## 4. Variables, Keywords, and Layout Surface

- Use the variables page as the source of truth for toggles and numeric options.
- Common families to reason about:
  - monitors
  - input
  - decoration
  - animations
  - group settings
  - misc behavior
  - bind-related options
  - OpenGL / XWayland / debug knobs when relevant
- Keep monitor placement and scaling explicit.
- Keep input behavior explicit for keyboard, touchpad, and pointer comfort.

## 5. Binds and Dispatchers

- Use the documented bind syntax for the detected family.
- Prefer common dispatchers when building keymaps:
  - `exec`
  - `killactive`
  - `togglefloating`
  - `fullscreen`
  - `movefocus`
  - `workspace`
  - `movetoworkspace`
  - `togglespecialworkspace`
  - `resizeactive`
- Use `pass` or `sendshortcut` only when a global shortcut flow is actually needed.
- Do not substitute made-up dispatcher names for convenience.
- Keep launcher, screenshot, and window management binds consistent with the rest of the desktop.

## 6. Startup and Session Wiring

- Use `exec-once` for idempotent session daemons:
  - panel or shell
  - wallpaper daemon
  - idle manager
  - lock helper
  - tray applets
  - polkit agent
  - notification daemon when needed
- Do not declare companion shell configs in the Hyprland config.
- When a component is selected, ensure the startup path is real:
  - a config file exists
  - an autostart line exists
  - any required environment variables are set
- Barebones setups should feel complete after login, not just installed.

## 7. Monitors and Input

- Keep monitor definitions explicit and stable.
- Prefer sensible defaults for:
  - scale
  - refresh rate
  - workspace assignment
  - gap sizing
- Make input behavior match the chosen desktop style:
  - tap behavior
  - natural scrolling
  - keyboard layout / repeat behavior
  - touchpad palm and tap settings when relevant

## 8. Animations and Decoration

- Use rounded corners, blur, and shadows in a way that matches the chosen visual language.
- Keep blur nested inside the proper decoration sub-blocks.
- Keep shadows in their own nested block if the target syntax expects it.
- Prefer restrained animation curves for polished rice setups.
- Avoid writing old decoration fields directly if the detected family expects a newer structure.

### Blur and Shadow Shape

- If the target syntax uses nested decoration blocks, keep blur settings inside `blur { ... }`.
- Do not place blur keys directly beside unrelated decoration keys when the version expects a nested block.
- Keep shadow settings in the matching shadow block for the installed syntax family.
- The generator should err on the side of explicit structure rather than abbreviated shorthand.

## 9. Window Rules

- Use the version-appropriate rule syntax.
- Newer syntax families prefer explicit `windowrule` style declarations with match prefixes.
- Older branches may still use the older window rule style for compatibility with that release family.
- Prefer explicit match prefixes for window selectors when the current syntax expects them.
- Keep rules focused:
  - float specific apps
  - center dialogs
  - assign workspaces intentionally
  - set opacity or rounding only where it improves the layout
- Avoid deprecated rule variants when the installed Hyprland family no longer accepts them.

## 10. Layer Rules

- Treat layer rules as a separate syntax surface from window rules.
- Use the modern namespace matching form when required by the version.
- Prefer explicit rule values such as `blur on` or `ignore_alpha 0` when the release family expects them.
- Keep blur and transparency rules explicit and reload-safe.

## 11. Practical Generation Checklist

- Choose the correct config family first.
- Generate the minimal working Hyprland config for the requested stack.
- Add startup wiring for every component that needs to launch.
- Keep wallpaper, lockscreen, launcher, panel, and notification behavior consistent with the rest of the desktop.
- Make edits additive when the user is refining an existing setup.
- Never place non-Hyprland config syntax inside Hyprland blocks.
