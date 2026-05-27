# ⚡ Dotengine

```
      ____        __                  _             
     / __ \____  / /____  ____  ____ _(_)___  ___   
    / / / / __ \/ __/ _ \/ __ \/ __ `/ / __ \/ _ \  
   / /_/ / /_/ / /_/  __/ / / / /_/ / / / / /  __/  
  /_____/\____/\__/\___/_/ /_/\__, /_/_/ /_/\___/   
                             /____/                 
```

**Let's be honest:** ricing your Hyprland desktop is one of the most satisfying things in the Linux world. But wrestling with deprecated syntaxes, dealing with status bars that won't start on reboot, and staring at those annoying unicode "tofu" blocks in your Waybar? Yeah, not so fun.

Enter **Dotengine** — your highly resilient, AI-powered Hyprland configuration and self-healing assistant CLI. 

Dotengine inspects your active host environment, dynamically recommends a premium component stack matching your visual prompt (or lets you customize it completely), synthesizes flawless config sheets, and automatically corrects syntaxes if reload warnings occur. It's ricing made smart!

---

## ✨ Features

*   **Dynamic Component Recommendations**: Resolves the best combination of Panel, Launcher, Notification Center, Wallpaper Switcher, and Lockscreens based on your natural language aesthetic choice.
*   **Complete Customization Flexibility**: Gives you full interactive menu override options to pick whichever tools you want:
    *   *Status Panels*: Aylur's GTK Shell (`ags`), `waybar`, `quick-shell`, or `none`.
    *   *App Launchers*: `rofi` or `none`.
    *   *Notification Center*: Sway Notification Center (`swaync`), `dunst`, or `none`.
    *   *Wallpaper Switcher*: `hyprpaper` (with dynamic transitional instructions) or `none`.
    *   *Lockscreens*: `hyprlock`, `swaylock`, `waylock`, or `none`.
*   **System Tray Connection Applets**: Integrated network (`nm-applet`) and Bluetooth (`blueman-applet`) checkups. They are automatically checked, installed if missing, and launched in the background at startup!
*   **Pre-Run Safe Backups**: Detects if you have a pre-existing Hyprland directory (`~/.config/hypr`) and prompts to recursively back up all your active config folders (Waybar, Rofi, SwayNC, Dunst, etc.) to a dated folder before any file changes start.
*   **Self-Healing Syntaxes**: If reload diagnostic warnings or failures occur, the CLI triggers a mock healing loop, reads the stderr warning diagnostics, and automatically repairs the config.

---

## 🚀 Installation

You can download and install the latest stable compiled release of `dotengine` with a single `curl` command:

```bash
curl -fsSL https://raw.githubusercontent.com/Aryan241006/dotengine/main/install.sh | bash
```

*Note: The script automatically detects your CPU architecture and OS platform, moves the executable binary safely into your path (such as `/usr/local/bin` or `~/.local/bin`), and handles binary privileges.*

---

## ⚙️ How to Use

### 1. Launch the CLI
Simply open your terminal and run `dotengine` to launch the interactive composer menu:
```bash
dotengine
```

### 2. Enter Design Prompts & Choose Profiles
Select one of the starting design templates or define your own prompt description from scratch:
*   `[0] Glassmorphic Glow` — Frosted-glass overlays, unified dashboard widgets. (recommends `ags`)
*   `[1] Nordic Minimalist` — Super clean Arctic-themed workspaces, high usability. (recommends `waybar` + `swaync`)
*   `[2] Retro Cyberpunk` — Neon magenta accents and swift scaling.

### 3. Customize Your Stack & Styling Prompts
*   **Dynamic Customization**: The CLI will recommend the best stack for your aesthetic. You will be prompted: `Would you like to customize this stack? [y/N]: `. Say `y` if you want to override any tool slot manually!
*   **Style Prompts**: You can describe exactly how the wallpaper and lockscreen should look (e.g. *"purple sunset minimalist mountains"*, or *"frosted glass login box with analog clock"*). These styling preferences are sent directly to the generative AI model to build custom files matching your vision.

---

## 🛡 Safe Pre-Run Backups

We know your configs are precious. If `dotengine` detects a pre-existing `~/.config/hypr` directory, it will prompt:
```
⚠ Existing Hyprland configuration detected at ~/.config/hypr.
Would you like to back up your existing desktop configurations before proceeding? [Y/n]:
```
If you confirm, a comprehensive dated backup folder will be created under `~/.local/share/dotengine/backups/pre-run-<timestamp>` storing all your old setups safely!

---

## 🧠 Core Blur Syntax Rules (Important!)

To avoid the infamous config reload failures, `dotengine` enforces the modern Hyprland decoration blur syntax. **Do not use colons or flat blur keys directly inside decoration blocks.** 

*   **Correct Syntax (Required by newer versions):**
    ```ini
    decoration {
        rounding = 12
        blur {
            enabled = true
            size = 8
            passes = 3
            new_optimizations = true
        }
    }
    ```
*   **Incorrect Syntax (Deprecated & causes crashes):**
    ```ini
    decoration {
        rounding = 12
        blur = true
        blur_size = 8
    }
    ```

---

## 🏗 Developing locally

To build the executable and run the test suite locally:

```bash
# Clone the repository
git clone https://github.com/Aryan241006/dotengine.git
cd dotengine

# Run the automated test suite (17 tests passing!)
cargo test

# Compile and run the CLI locally
cargo run -- --help
```

---

*Made with ⚡ by Aryan and the dotengine team.*
