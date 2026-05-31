# Hyprland Design Skill

This document is the visual and composition reference for Dotengine. It should help the generator understand what a Hyprland rice is trying to feel like, how complete setups are wired, and how to transform a screenshot into a usable desktop plan.

## 1. What A Good Setup Must Feel Like

- Finished, not just installed.
- Consistent across wallpaper, panel, launcher, lockscreen, notifications, and tray helpers.
- Balanced between visual identity and daily usability.
- Clearly wired on first login:
  - wallpaper appears
  - panel starts
  - lockscreen works
  - idle behavior is defined
  - portals and agents are not forgotten

## 2. Reference-Design Interpretation

- Treat screenshots as design intent, not as a pixel-perfect contract.
- Infer:
  - palette
  - contrast level
  - blur strength
  - rounding
  - density
  - spacing rhythm
  - launcher style
  - panel shape
  - notification style
  - lockscreen mood
  - wallpaper behavior
  - startup completeness
- If the screenshot implies a fuller desktop, wire the missing plumbing instead of only copying the visible surface.

## 3. Design Archetypes

### Glassmorphic Glow

- High blur
- Rounded cards
- Layered transparency
- Soft borders and ambient shadows
- Best when paired with an interactive shell, a modern launcher, and a polished lockscreen

### Minimalist Arctic

- Light visual weight
- Clean typography
- Tight module spacing
- Strong contrast
- Best for Waybar-style panels and low-overhead notification setups

### Fluid Interactive

- Motion-forward widgets
- Dashboard-like shells
- Layered panels and overlays
- Good for setups that want richer interaction without looking crowded

### Terminal-First Functional

- Sparse but deliberate
- Highly legible
- Compact launcher and panel
- Minimal noise
- Focus on system clarity and speed

## 4. Example-Config Pattern Library

Use the official example configuration page and the referenced dotfiles as pattern sources for layout, file structure, and integration depth:

- `end-4/dots-hyprland`
- `mylinuxforwork/dotfiles`
- `fufexan/dotfiles`
- `linuxmobile/hyprland-dots`
- `flick0/dotfiles`
- `1amSimp1e/dots`
- `notusknot/dotfiles-nix`
- `coffebar/dotfiles`

What to learn from these repositories:

- how complete rice stacks split responsibilities across files
- how startup daemons and shell components are launched
- how panel config, CSS, and scripts work together
- how lockscreen and wallpaper behavior are kept visually aligned
- how launcher, notification, and tray workflows are made part of the desktop instead of optional extras
- how reproducibility and theming scale from single-file configs to multi-directory setups

## 5. Common File Families To Expect

- Hyprland config:
  - `hyprland.conf`
  - `hyprland.lua`
- Panel or shell:
  - Waybar config and CSS
  - AGS JavaScript/TypeScript and stylesheet files
  - Quickshell/QML modules
- Launcher:
  - Rofi `rasi`
  - other launcher config directories when selected
- Notification layer:
  - SwayNC JSON and CSS
  - Dunst config when used instead
- Lockscreen:
  - Hyprlock config
- Wallpaper:
  - Hyprpaper config
  - or another wallpaper daemon config when selected
- Support assets:
  - theme variables
  - fonts
  - GTK settings
  - scripts
  - user service files

## 6. Completion Rules

- A design is incomplete if it only changes colors but does not wire startup behavior.
- A design is incomplete if it only installs packages without creating configs.
- A design is incomplete if the desktop cannot explain what starts on login.
- A design is incomplete if wallpaper, lockscreen, panel, and notifications do not align visually and functionally.
- A barebones Hyprland install should be turned into a usable desktop, not a set of disconnected apps.

## 7. Visual Guidance

- **AVOID GENERIC OUTLINES**: Do NOT rely on heavy borders or generic colored outlines around windows, active panels, or buttons unless explicitly requested. Modern, state-of-the-art designs favor borderless blocks, soft drop-shadows, subtle transparencies, and glassmorphism. Set border/outline sizes to `0` or keep them extremely subtle (`1px` with a highly transparent overlay color).
- Prefer purposeful color systems over random accent colors (e.g. utilize Catppuccin, Gruvbox, Nord, or Tokyo Night color mappings with 0.8-0.9 opacity for blurred windows).
- Use blur, opacity, and rounding as part of a single visual language.
- Keep panel and launcher styling aligned with the wallpaper and lockscreen tone.
- Avoid hollow setups that look themed in screenshots but lack startup wiring or companion services.

## 8. Premium Component Styling Blueprints

### A. Rofi Premium Styling Blueprint
To achieve a modern, premium appearance and completely bypass the system's ugly fallback brownish/yellowish outlines, you MUST prepend `@theme "/dev/null"` to `/config.rasi` and define clean, padded layout elements:
```css
configuration {
    modi: "drun";
    show-icons: true;
    icon-theme: "Papirus";
    drun-display-format: "{name}";
    disable-history: false;
    hide-scrollbar: true;
    display-drun: "   ";
    sidebar-mode: false;
}

@theme "/dev/null"

* {
    bg-col:  #1e1e2e;
    bg-col-light: #1e1e2e;
    border-col: #89b4fa; /* Accent color */
    selected-col: #89b4fa;
    fg-col: #cdd6f4;
    fg-col2: #a6adc8;
    width: 600;
    font: "JetBrainsMono Nerd Font 12";
}

element-text, element-icon , mode-switcher {
    background-color: inherit;
    text-color:       inherit;
}

window {
    height: 450px;
    border: 2px;
    border-color: @border-col;
    background-color: @bg-col;
    border-radius: 16px;
}

mainbox {
    background-color: @bg-col;
}

inputbar {
    children: [prompt,entry];
    background-color: @bg-col;
    border-radius: 5px;
    padding: 2px;
}

prompt {
    background-color: @selected-col;
    padding: 8px;
    text-color: #11111b;
    border-radius: 8px;
    margin: 10px 0px 10px 10px;
}

entry {
    padding: 8px;
    margin: 10px 10px 10px 0px;
    text-color: @fg-col;
    background-color: rgba(30, 30, 46, 0.5);
    border-radius: 8px;
    placeholder: "Search applications...";
    placeholder-color: @fg-col2;
}

listview {
    border: 0px 0px 0px;
    padding: 6px 0px 0px;
    margin: 10px 10px 0px 10px;
    columns: 1;
    lines: 8;
    background-color: @bg-col;
}

element {
    padding: 10px;
    background-color: @bg-col;
    text-color: @fg-col;
    border-radius: 8px;
    margin: 2px 0px;
}

element-icon {
    size: 28px;
    margin: 0 10px 0 0;
}

element selected {
    background-color:  @selected-col;
    text-color: #11111b;
}
```

### B. Waybar Premium Styling Blueprint
Rely on capsule (pill-shaped) layouts and glassmorphic transparency rather than boxy outline-heavy structures:
- **Pill Modules**: Group modules into pill-shaped containers with rounded corners (`border-radius: 15px`), padding (`padding: 2px 12px`), margins (`margin: 4px`), and elegant semi-transparent backgrounds (`background-color: rgba(255, 255, 255, 0.05);` or matching theme color).
- **Glassmorphic Bar**: Style the main bar window with standard CSS opacity and margins to float elegant panels:
  ```css
  window#waybar {
      background-color: rgba(30, 30, 46, 0.85); /* transparent background */
      border-radius: 16px;
      margin: 10px 16px 0px 16px;
  }
  ```
- **Custom Glyphs**: Bind custom, premium Nerd Font icons (`` clock, `` battery, `` wifi, `` audio, `` power) instead of text blocks.


