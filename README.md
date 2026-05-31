# Dotengine

```
 ___   ___ _____ ___ _  _  ___ ___ _  _ ___ 
|   \ / _ \_   _| __| \| |/ __|_ _| \| | __|
| |) | (_) || | | _|| .` | (_ || || .` | _| 
|___/ \___/ |_| |___|_|\_|\___|___|_|\_|___|
```

**Dotengine** is an AI-powered desktop configuration and self-healing agent CLI for Hyprland. Built in Rust utilizing Clean/Hexagonal Architecture, it allows users to dynamically generate, apply, and auto-correct visual desktop environments in real time using state-of-the-art generative language models.

---

## 🚀 How it Works (Workflows)

Dotengine leverages a dual-workflow pipeline designed to transition your desktop from a blank slate to a highly stylized, functionally complete workspace without manual troubleshooting:

```
[ User Prompt / Mockup ] ──> [ 1. Generation Workflow ] ──> [ Write Files ] ──> [ 2. Verification / Reload ]
                                                                                   │
                                   [ Flawless Configs ] <── [ Success ] <──────────┼──> [ Failure / Diagnostics ]
                                                                                          │
                                                                                    [ 3. Self-Healing Workflow ]
```

### 1. The Generation Workflow
When you describe your desired aesthetic (or supply an optional screenshot mockup):
*   **Aesthetic Heuristics**: The CLI analyzes your prompt and determines the optimal suite of status bars, launchers, lockscreens, and system applets.
*   **Prerequisite Verification**: It checks if the recommended tools are present in your system path and offers to install missing ones using your native package manager (Arch, Ubuntu, Fedora, etc.).
*   **Generative AI Synthesis**: It queries the selected AI model, supplying precise design specifications, technical shell rules, monitor layouts, and system packages to synthesize a complete configuration.

### 2. The Self-Healing Workflow
Unlike typical script templates, Dotengine provides active run-time verification:
*   **Active Verification**: It applies the generated configs and reloads the display server using `hyprctl reload`.
*   **Diagnostic Parsing**: If any syntax warnings or configuration errors occur, the CLI catches the `stderr` diagnostics, isolates the offending lines, and automatically initiates a self-healing loop.
*   **Iterative Healing**: The diagnostic payload is fed back to the AI model to regenerate the corrected lines. This process runs iteratively until your desktop loads warning-free.

---

## 🧠 AI Integration & Model Support

Dotengine uses LLMs to interpret natural language, analyze design mockup screenshots, write highly precise configuration syntax, and debug its own reload diagnostics.

### Supported Models
You can choose between two primary AI providers directly from the CLI:
*   **Gemini**: Utilizes `gemini-3.5-flash` for high-speed, multimodal layout synthesis.
*   **OpenAI**: Supports `gpt-4o` and `gpt-4o-mini` for highly reliable code generation and text reasoning.

### Keys Setup
On your first run, the CLI will interactively ask you to select your preferred provider and paste your API key. Keys are securely saved locally inside your secure config directory (`~/.config/dotengine/credentials.json`) and are encrypted/read silently on all subsequent runs.

Alternatively, you can supply your API keys directly via environment variables to bypass prompting and credentials files:
*   `GEMINI_API_KEY` for Gemini provider.
*   `OPENAI_API_KEY` for OpenAI provider.

---

## ✨ Core Desktop Stack Support

Dotengine dynamically configures and integrates the following essential components:
*   **Status Panel / Shell**: Aylur's GTK Shell (`ags`), `waybar`, `quick-shell`, or `none`.
*   **App Launcher**: `rofi` or `none`.
*   **Notification Center**: Sway Notification Center (`swaync`), `dunst`, or `none`.
*   **Wallpaper Switcher**: `hyprpaper` (includes automatic transitions and a post-run usage guide) or `none`.
*   **Lockscreen Tool**: `hyprlock`, `swaylock`, `waylock`, or `none`.
*   **System Tray Applets**: Connection managers like Network Manager (`nm-applet`) and Bluetooth (`blueman-applet`) are automatically checked, installed, and launched at startup.

---

## 🛡 Pre-Run Secure Backups

To ensure your existing configuration files are safe, Dotengine performs a pre-run check. If a pre-existing Hyprland directory is found under `~/.config/hypr`, the CLI will prompt you:
```
⚠ Existing Hyprland configuration detected at ~/.config/hypr.
Would you like to back up your existing desktop configurations before proceeding? [Y/n]:
```
Confirming will securely copy `.config/hypr`, `.config/waybar`, `.config/rofi`, `.config/dunst`, `.config/ags`, `.config/swaync`, and `.config/quickshell` into a dated restore point at `~/.local/share/dotengine/backups/pre-run-<timestamp>`.

---

## 📦 Installation

To download and install the pre-compiled executable binary in one command, run:

```bash
curl -fsSL https://raw.githubusercontent.com/Aryan241006/dotengine/main/install.sh | bash
```

*The installer script automatically detects your CPU architecture and OS platform, moves the executable safely into your path, and handles execution permissions.*

---

## ⚙️ Quick Start

```bash
# 1. Run the interactive composer menu
dotengine

# 2. Advanced automation: Run with manual stack arguments
dotengine --prompt "Nord minimalist arctic theme" --panel waybar --launcher rofi --wallpaper hyprpaper --lockscreen hyprlock
```

### Developing Locally
If you prefer to compile the Rust binary from source:
```bash
# Clone the repository
git clone https://github.com/Aryan241006/dotengine.git
cd dotengine

# Run the test suite (51 unit and integration tests passing)
cargo test

# Compile and run the CLI
cargo run -- --help
```
