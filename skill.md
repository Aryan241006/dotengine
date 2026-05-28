# Hyprland Distro-Aware Technical Configuration Syntax (skill.md)

This document defines the strict syntax requirements, dispatcher commands, and layout blocks for generating correct and warning-free Hyprland configurations based on system versioning.

---

## 1. Version-Based File Hierarchy

*   **Hyprland v0.55.0 and above**:
    *   Uses **Lua-based configurations**.
    *   File must be saved as: `.config/hypr/hyprland.lua`
    *   If `hyprland.lua` exists, Hyprland loads it directly. You can split configurations and load them using Lua's standard `require()` syntax.
*   **Hyprland v0.54.0 and below** (e.g., v0.53.3 standard in Ubuntu 26.04 LTS):
    *   Uses classic **Hyprlang configuration syntax**.
    *   File must be saved as: `.config/hypr/hyprland.conf`

---

## 2. Unified Window Rules (Hyprland v0.53.0 and above)

*   **Keyword Deprecation**: The `windowrulev2` keyword is **completely deprecated and removed**. You MUST use the `windowrule` keyword for all rules.
*   **Match Prefixes Required**: Every window matching filter (such as `class`, `title`, `tag`, `initialClass`, or `initialTitle`) must be explicitly prefixed with `match:`.
*   **Explicit Flag Values**: Boolean flags (such as `float`, `center`, `pin`, `fullscreen`) now require an explicit integer value (`1` for true/enable, `0` for false/disable).
*   **Old vs. New Syntax Examples**:
    *   *Incorrect (Deprecated):* `windowrulev2 = float, class:^(kitty)$`
    *   *Correct (Modern):* `windowrule = float 1, match:class ^(kitty)$`
    *   *Incorrect (Deprecated):* `windowrulev2 = opacity 0.95, class:^(vscode)$`
    *   *Correct (Modern):* `windowrule = opacity 0.95, match:class ^(vscode)$`

---

## 3. Decoration Effects & Shadow Blocks (Hyprland v0.42.0 and above)

*   **Shadow Sub-Block Required**: Defining shadow properties (like `drop_shadow`, `shadow_range`, `col.shadow`) directly within the `decoration` block is **deprecated and triggers configuration errors**.
*   **Blur Sub-Block Required**: Defining blur properties (like `blur = true`, `blur_size = 8`, `blur_passes = 3`) directly within the `decoration` block is **deprecated and causes syntax load/reload errors**. Colons like `decoration:blur` are also invalid. All blur properties **MUST** be defined in a nested `blur { ... }` sub-block.
*   **Structured Format**: All shadow and blur parameters must be encapsulated in their respective nested blocks inside `decoration`.
*   **Old vs. New Syntax Examples**:
    *   *Incorrect (Deprecated / Invalid Syntax):*
        ```ini
        decoration {
            rounding = 14
            blur = true
            blur_size = 8
            drop_shadow = true
            shadow_range = 20
        }
        ```
    *   *Correct (Modern / Required Syntax):*
        ```ini
        decoration {
            rounding = 14
            active_opacity = 0.95
            inactive_opacity = 0.90
            
            blur {
                enabled = true
                size = 8
                passes = 3
                new_optimizations = true
            }
            
            shadow {
                enabled = true
                range = 20
                render_power = 4
                color = rgba(11111bee)
            }
        }
        ```

---

## 4. Modern Layer Rules (layerrule) Syntax (Hyprland v0.53.0 and above)

*   **Explicit Match Prefix Required**: Positional comma-separated layer rules (e.g. `layerrule = blur, waybar`) are **completely deprecated and trigger syntax errors** (e.g. `invalid field blur: missing a value`). You MUST explicitly prefix the layer namespace with `match:namespace`.
*   **Explicit Flag Values & Replaced Fields**: 
    *   `blur` requires an explicit value (like `on`).
    *   `ignorezero` has been **completely removed** and must be replaced with `ignore_alpha 0` (note the underscore).
*   **Correct Syntax Examples**:
    *   *Incorrect (Deprecated / Syntax Error):*
        ```ini
        layerrule = blur, waybar
        layerrule = ignorezero, waybar
        ```
    *   *Correct (Modern / Required Syntax):*
        ```ini
        layerrule = blur on, match:namespace ^(waybar)$
        layerrule = ignore_alpha 0, match:namespace ^(waybar)$
        ```

---

## 5. Keybinding Dispatchers & Key Rules

*   **Dispatcher for Floating**: To toggle a window between floating and tiling, you must use the **`togglefloating`** dispatcher. Never use `togglefloat` or `float` as a dispatcher.
    *   *Incorrect:* `bind = SUPER, V, togglefloat`
    *   *Correct:* `bind = SUPER, V, togglefloating`
*   **Dispatcher for Closing Active Windows**: Always use **`killactive`** to close/terminate the active window.
    *   *Correct:* `bind = SUPER, C, killactive`
