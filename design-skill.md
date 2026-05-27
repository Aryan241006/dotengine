# Hyprland & Desktop Shell Design Principles (design-skill.md)

This document outlines the core design systems, color palettes, visual hierarchies, software integrations, and advanced ricing guidelines for generating highly modern, visually striking, and functionally complete Hyprland environments.

---

## 1. Modern Color Palettes

A gorgeous UI starts with a meticulously curated color palette. Avoid default or high-saturation primaries.

### Catppuccin Mocha (Dark Pastel theme)
- **Rosewater**: `#f5e0dc`
- **Flamingo**: `#f2cdcd`
- **Pink**: `#f5c2e7`
- **Mauve**: `#cba6f7` (Primary Accent)
- **Red**: `#f38ba8` (Danger/Error)
- **Peach**: `#fab387` (Warning)
- **Yellow**: `#f9e2af`
- **Green**: `#a6e3a1` (Success)
- **Teal**: `#94e2d5`
- **Sky**: `#89dceb`
- **Sapphire**: `#74c7ec`
- **Blue**: `#89b4fa` (Secondary Accent)
- **Lavender**: `#b4befe`
- **Base (Background)**: `#1e1e2e`
- **Mantle (Sub-bg)**: `#181825`
- **Crust (Deep-bg)**: `#11111b`

### Nord (Cool Arctic theme)
- **Polar Night (Backgrounds)**: `#2e3440` (Nord0), `#3b4252` (Nord1), `#434c5e` (Nord2)
- **Snow Storm (Foregrounds)**: `#d8dee9` (Nord4), `#e5e9f0` (Nord5), `#eceff4` (Nord6)
- **Frost (Accents/Highlights)**: `#8fbcbb` (Nord7), `#88c0d0` (Nord8), `#81a1c1` (Nord9), `#5e81ac` (Nord10)
- **Aurora (Indicators)**: `#bf616a` (Red), `#d08770` (Orange), `#ebcb8b` (Yellow), `#a3be8c` (Green), `#b48ead` (Purple)

### Tokyo Night (Cyberpunk Dark theme)
- **Background**: `#1a1b26`
- **Terminal Background**: `#16161e`
- **Foreground**: `#c0caf5`
- **Blue (Accent)**: `#7aa2f7`
- **Cyan**: `#7dcfff`
- **Magenta**: `#bb9af7`
- **Red**: `#f7768e`
- **Orange**: `#ff9e64`
- **Yellow**: `#e0af68`
- **Green**: `#9ece6a`

---

## 2. Software Architecture Suggestions

When designing an aesthetic, dotengine suggests the best software combinations based on user visual choices:

| Design Archetype | Visual Features | Recommended Stack | Rationale & Recommendation |
| :--- | :--- | :--- | :--- |
| **Glassmorphic Glow** | Vibrant blur, rounded notifications, dynamic dashboard | **AGS (Aylurs GTK Shell)** + **Rofi** | **Highly Recommended**. AGS handles JS/TS scripting, allowing dynamic widgets, unified audio controllers, and premium glass filters (`backdrop-filter`) that other options cannot match easily. |
| **Minimalist Arctic** | Clean flat widgets, thin borders, high contrast, low overhead | **Waybar** + **SwayNC** + **Rofi** | Recommended for low-spec or keyboard-centric workflows. Simple CSS styling via Waybar and modern widget layout cards via SwayNC fit the blocky Nord vibe seamlessly. |
| **Fluid Interactive** | High framerate widgets, fluid custom overlays, QML layouts | **Quick Shell** + **SwayNC** + **Rofi** | Excellent alternative for developers wanting QML-based UI widgets with high-performance animations and custom state management. |

---

## 3. Style Sheets & Layout Guidelines

### Hyprland Config Rules (`hyprland.conf`)
- **Gaps**: Use internal gaps of `5px` to `8px` and outer gaps of `10px` to `15px` to allow wallpaper visibility.
- **Rounding**: Set `decoration:rounding = 10` to `14` for smooth, modern curves matching standard GTK windows.
- **Shadows**: Enable shadows with high blur (`decoration:shadow:range = 15`, `decoration:shadow:render_power = 3`, `decoration:shadow:color = rgba(1a1a1aee)`).
- **Blur**: Use dual-cavity blur for glass panels (`decoration:blur:enabled = true`, `size = 8`, `passes = 3`, `new_optimizations = true`, `xray = true`).
- **Animations**: Prefer custom bezier curves for springy windows:
  ```ini
  bezier = wind, 0.05, 0.9, 0.1, 1.05
  bezier = winShrink, 0.15, 0.85, 0.1, 1.1
  animation = windows, 1, 6, wind, slide
  animation = windowsIn, 1, 6, winShrink, slide
  animation = border, 1, 10, default
  animation = fade, 1, 7, default
  animation = workspaces, 1, 5, wind, slide
  ```

### AGS Styling (SCSS)
Use variables matching the chosen palette, applying CSS grid and box-model rules.
- **Glass Panel Style**:
  ```css
  .glass-panel {
      background-color: rgba(30, 30, 46, 0.65); /* 65% opacity base */
      border: 1px solid rgba(255, 255, 255, 0.08); /* fine border */
      border-radius: 12px;
      box-shadow: 0 4px 30px rgba(0, 0, 0, 0.2);
  }
  ```

### Rofi Styling (`.rasi`)
- Avoid old standard layouts. Use centered prompt overlay with input lines, matching modern app launchers.
- Enable `blur` on the Rofi window if backed by Hyprland's `blurls = rofi`.
- Config fragment:
  ```rasi
  * {
      bg: #1e1e2eCC; /* transparent */
      fg: #cdd6f4;
      accent: #cba6f7;
      border-color: rgba(255, 255, 255, 0.1);
  }
  window {
      background-color: @bg;
      border: 1px;
      border-color: @border-color;
      border-radius: 12px;
      width: 30%;
  }
  ```

---

## 4. SwayNC Notification Center CSS & Custom Layouts

SwayNC (Sway Notification Center) is a CSS-styled notification manager which provides a control center-style panel:
- **Nested Structure**: Config files must be saved under `.config/swaync/config.json` and `.config/swaync/style.css`.
- **Frosted Glass Cards**: Styling rules must apply semi-transparent background colors paired with rounded borders (`border-radius: 10px` to `14px`) and matching box shadows to fit the global desktop aesthetic.
- **SwayNC Glass styling example**:
  ```css
  .notification {
      background: rgba(30, 30, 46, 0.75);
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 12px;
      color: #cdd6f4;
      margin: 6px;
      padding: 10px;
  }
  .notification:hover {
      background: rgba(45, 45, 70, 0.85);
  }
  .control-center {
      background: rgba(24, 24, 37, 0.85);
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 16px;
  }
  ```

---

## 5. System Tray & Connection Managers Applets

Tray applets bridge the keyboard-centric tiling workflow with quick mouse connectivity actions:
*   **Startup execution**: Launchers for connection panels MUST be daemonized in `hyprland.conf`:
    ```ini
    exec-once = nm-applet --indicator
    exec-once = blueman-applet
    ```
*   **Aesthetic binding shortcuts**: Assign intuitive shortcuts in `hyprland.conf` to easily open advanced system manager dialog sheets on-the-fly:
    ```ini
    # Open Network Connections GUI editor
    bind = SUPER SHIFT, N, exec, nm-connection-editor
    # Open Blueman Manager panel
    bind = SUPER SHIFT, B, exec, blueman-manager
    ```

---

## 6. Advanced Complex Ricing Concepts

For rices demanding breathtaking visual choreography and state-of-the-art responsiveness:

### A. Dynamic Wallpaper Systems (`swww`)
Instead of static wallpaper loaders, use `swww` for smooth, high-fidelity dynamic visual sweeps:
- **Installation requirement**: Check/install package `"swww"`.
- **Execution rule**: Start the `swww-daemon` on login and load wallpapers with sliding transition sweeps:
  ```ini
  exec-once = swww-daemon
  exec-once = swww img /path/to/wallpaper.png --transition-type wave --transition-angle 30 --transition-step 90
  ```

### B. Dynamic Palette Synchronization (`Pywal` / `Material 3`)
- **Theme Matching**: The generated styles should automatically read the active wallpaper dominant tones and synchronize status panels (Waybar), launchers (Rofi), and terminals (Kitty/Foot) using toolsets like `pywal` or custom Python palette generators.

### C. GTK 3/4 dynamic scaling
To avoid jarring visual gaps between client windows and native widgets, synchronize GTK templates:
- Apply theme stylesheets (`.config/gtk-3.0/settings.ini`, `.config/gtk-4.0/settings.ini`) to force color scheme settings (`prefer-dark`), matching icon-themes (like `Papirus-Dark`), and layout fonts (like `Inter`).
