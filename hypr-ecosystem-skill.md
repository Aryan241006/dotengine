# Hypr Ecosystem Skill

This document teaches Dotengine how to reason about the Hypr ecosystem as desktop infrastructure instead of as a list of optional packages.

## 1. Planning Rule

- Treat ecosystem tools as parts of a desktop graph:
  - some must be configured
  - some must be started
  - some are libraries only
  - some are support utilities that should be present when their dependent stack is selected
- Do not recommend a component without also checking whether it needs config, startup wiring, theme integration, or idle/portal support.

## 2. User-Facing Utilities

### hyprpaper

- Wallpaper daemon and wallpaper selection flow.
- Needs a real config file (`.config/hypr/hyprpaper.conf`) and an `exec-once = hyprpaper` startup line in `hyprland.conf` when used.
- **CRITICAL CONFIGURATION SYNTAX (v0.8.x+)**: Newer versions of `hyprpaper` (used on modern systems) use block-based syntax. Do NOT write legacy inline properties (like `wallpaper = monitor,path`). Always use the modern `wallpaper` block structure:
  ```ini
  preload = /home/USERNAME/.config/hypr/wallpaper.png

  wallpaper {
      monitor = 
      path = /home/USERNAME/.config/hypr/wallpaper.png
      fit_mode = cover
  }
  ```
  - **Wildcard Monitors**: Leaving the `monitor = ` line empty acts as a wildcard, applying the wallpaper to all connected monitors automatically.
  - **No Tildes**: Do NOT use tildes (`~`) in paths inside `hyprpaper.conf`, as `hyprpaper` does not support path expansion. Always use absolute paths.
- Should be paired with wallpaper-aware theme guidance so the rice does not feel empty.


### hyprpicker

- Color picker utility for deriving palette values from the desktop or a reference image.
- Wire it to a keybind or launcher action when the user wants color sampling during ricing.

### hyprlauncher

- Treat as a launcher front-end when selected.
- Generate its config and connect it to a startup/binding flow if the desktop stack depends on it.

### hyprlock

- Lockscreen companion.
- Needs its own config file (`.config/hypr/hyprlock.conf`) and should match the wallpaper, palette, and typography of the rest of the rice.
- **CRITICAL**: Always map a manual lock shortcut in `hyprland.conf`:
  `bind = SUPER, L, exec, hyprlock` (or `swaylock` / `waylock` depending on the lockscreen utility in use).
- Pair it with idle logic (`hypridle`) so the lockscreen is not just installed but actually reachable.

### hypridle

- Idle session manager.
- Should be wired to lock, dim, suspend, or other idle-sensitive actions.
- It is a core part of a complete barebones login flow when the desktop uses Hyprlock.
- Needs its config file under `.config/hypr/hypridle.conf`.
- Structuring `hypridle.conf` format:
  ```ini
  general {
      lock_cmd = pidof hyprlock || hyprlock       # dbus/loginctl lock command
      before_sleep_cmd = loginctl lock-session    # lock before suspend
  }

  listener {
      timeout = 300                               # 5 minutes
      on-timeout = loginctl lock-session          # lock screen
  }
  ```
- Always autostart it on boot inside `hyprland.conf`:
  `exec-once = hypridle`

### wlogout

- A gorgeous, overlay-driven session and power menu overlay.
- Configuration layout must be written to `.config/wlogout/layout`.
  **CRITICAL LAYOUT SYNTAX**: `wlogout` does NOT parse a standard JSON array. Do NOT wrap the layout in `[ ... ]` and do NOT separate button objects with commas `,`. You MUST write them as separate, concatenated JSON objects one after another:
  ```json
  {
      "label": "lock",
      "action": "hyprlock",
      "text": "Lock (L)",
      "keybind": "l"
  }
  {
      "label": "logout",
      "action": "hyprctl dispatch exit",
      "text": "Exit (E)",
      "keybind": "e"
  }
  {
      "label": "shutdown",
      "action": "systemctl poweroff",
      "text": "Shutdown (S)",
      "keybind": "s"
  }
  {
      "label": "reboot",
      "action": "systemctl reboot",
      "text": "Reboot (R)",
      "keybind": "r"
  }
  ```

- Configuration styles must be written to `.config/wlogout/style.css`.
  **CRITICAL**: You MUST define custom background-image selectors for each button ID (e.g. `#lock`, `#logout`, `#shutdown`, `#reboot`) referencing `/usr/share/wlogout/icons/` paths. If you set `background-image: none;` on a wildcard (`*`), you must explicitly re-bind button icons, otherwise wlogout buttons will render completely blank and invisible!
  Use this template structure for `wlogout/style.css`:
  ```css
  * {
      background-image: none;
      transition: 200ms;
  }
  window {
      background-color: rgba(12, 12, 14, 0.85); /* matching dark background */
  }
  button {
      color: #cdd6f4;
      background-color: rgba(30, 30, 46, 0.9);
      border: 1px solid rgba(137, 180, 250, 0.2);
      border-radius: 12px;
      margin: 10px;
      background-repeat: no-repeat;
      background-position: center;
      background-size: 25%;
  }
  button:focus, button:hover {
      background-color: #89b4fa; /* accent color from palette */
      color: #11111b;
      border: 1px solid #89b4fa;
  }
  #lock {
      background-image: image(url("/usr/share/wlogout/icons/lock.png"));
  }
  #logout {
      background-image: image(url("/usr/share/wlogout/icons/logout.png"));
  }
  #shutdown {
      background-image: image(url("/usr/share/wlogout/icons/shutdown.png"));
  }
  #reboot {
      background-image: image(url("/usr/share/wlogout/icons/reboot.png"));
  }
  ```
- **CRITICAL WAYBAR COUPLING**: When waybar's panel is active and a session/power button is needed, always map its custom power module to execute `wlogout` on-click:
  ```json
  "custom/power": {
      "format": "",
      "on-click": "wlogout",
      "tooltip": false
  }
  ```




### xdg-desktop-portal-hyprland

- Portal integration required for a sane Wayland desktop experience.
- Include it in the setup when screen sharing, file pickers, or sandboxed desktop app integration matters.
- It is not a visual component, but it is part of a complete desktop.

### hyprsysteminfo

- System information utility.
- Use it when the design wants a dedicated system status view or shell element.

### hyprsunset

- Night light / color temperature tool.
- Include it when the desktop design includes time-based color mood or comfort adjustments.

### hyprpolkitagent

- Polkit authentication agent.
- A practical desktop needs a polkit agent when GUI admin prompts should work correctly.
- Start it as part of the session when selected.

### hyprland-qt-support

- Qt integration support for Hyprland-fluent theming.
- Use it when the stack includes Qt apps and the design wants them to match the desktop better.

### hyprqt6engine

- Qt 6 support helper for ecosystem integration.
- Treat as a compatibility / theming piece, not a visual app.

### hyprpwcenter

- Companion utility that should be treated like an ecosystem surface, not a random package suggestion.
- Only surface it when the selected desktop pattern has a clear use for it.

### hyprshutdown

- Session and power control helper.
- Include it when the rice uses a power menu or a desktop shutdown / logout surface.

## 3. Support Libraries And Toolkits

These are not autostart targets, but they matter when selecting packages and explaining dependencies:

- `hyprtoolkit`
- `hyprcursor`
- `hyprutils`
- `hyprlang`
- `hyprwayland-scanner`
- `aquamarine`
- `hyprgraphics`
- `hyprland-guiutils`

Rules for libraries:

- include them when a chosen app or stack depends on them
- do not create startup rules for them
- do not treat them as visible desktop components
- do not omit them from dependency reasoning when the selected stack clearly needs them

## 4. Ecosystem Wiring Checklist

- If wallpaper is selected, verify that a config file and startup launch exist.
- If lockscreen is selected, verify that idle handling can actually trigger it.
- If panel or shell is selected, verify autostart and matching theme assets.
- If tray or auth helpers are selected, verify they are launched in the session.
- If Qt theming support is selected, align it with the design system instead of leaving it default.
- If portals are missing, call it out explicitly because the desktop will feel incomplete.

## 5. Completeness Rules

- Installed but unwired is not complete.
- Wired but unthemed is not complete when the rest of the rice depends on visual matching.
- Visual matching without portal or idle support is not complete.
- A barebones Hyprland install should emerge with a functional wallpaper, lockscreen, panel, launcher, and portal path when the selected stack asks for them.

