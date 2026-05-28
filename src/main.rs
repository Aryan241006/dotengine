use clap::Parser;
use dotengine::application::{GenerationWorkflow, HealingWorkflow};
use dotengine::composer::compose;
use dotengine::domain::{DesignTemplate, ImagePayload};
use dotengine::infrastructure::{AiProvider, CredentialStore, GeminiClient, HyprSys, OpenaiClient};
use dotengine::ports::{AiService, SystemManager};
use dotengine::ui::{accent, heading, success, info, warning, error, activity};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "dotengine")]
#[command(author = "Aryan <aryan@dotengine.ai>")]
#[command(version)]
#[command(about = "Hyprland AI Configuration & Self-Healing Agent CLI", long_about = None)]
struct Args {
    /// Natural language prompt describing desired desktop aesthetic (e.g. 'nord themed minimal')
    #[arg(short, long)]
    prompt: Option<String>,

    /// Path to a UI design mockup image (JPG/PNG) for multi-modal generation
    #[arg(short, long)]
    image: Option<PathBuf>,

    /// Index of predefined design template (0: Glassmorphic Glow, 1: Nordic Minimalist, 2: Retro Cyberpunk)
    #[arg(short, long)]
    template: Option<usize>,

    /// AI service to use: 'openai' or 'gemini'. Overrides the saved default provider.
    #[arg(short, long)]
    model: Option<String>,

    /// Custom keybinding to open Rofi (e.g., 'SUPER, D' or 'SUPER, R')
    #[arg(long)]
    rofi_bind: Option<String>,

    /// Preferred status panel/shell ('ags', 'waybar', 'quick-shell', or 'none')
    #[arg(long)]
    panel: Option<String>,

    /// Preferred launcher ('rofi' or 'none')
    #[arg(long)]
    launcher: Option<String>,

    /// Preferred wallpaper utility ('hyprpaper' or 'none')
    #[arg(long)]
    wallpaper: Option<String>,

    /// Preferred lockscreen utility ('hyprlock', 'swaylock', 'waylock', or 'none')
    #[arg(long)]
    lockscreen: Option<String>,

    /// Preferred notification panel ('swaync', 'dunst', or 'none')
    #[arg(long)]
    notification: Option<String>,

    /// Enable Network Manager system tray applet (nm-applet)
    #[arg(long)]
    nm_applet: bool,

    /// Enable Bluetooth system tray applet (blueman-applet)
    #[arg(long)]
    blueman: bool,

    /// Custom prompt/aesthetic description for the wallpaper switcher
    #[arg(long)]
    wallpaper_prompt: Option<String>,

    /// Custom prompt/aesthetic description for the lockscreen design
    #[arg(long)]
    lockscreen_prompt: Option<String>,
}

fn heuristic_stack_for_prompt(prompt: &str, template_idx: Option<usize>) -> (String, String, String, String, String) {
    let predefined_templates = DesignTemplate::get_predefined_library();
    
    let resolved_idx = if let Some(idx) = template_idx {
        if idx < predefined_templates.len() {
            Some(idx)
        } else {
            None
        }
    } else {
        None
    };

    if let Some(idx) = resolved_idx {
        let template_name = &predefined_templates[idx].name;
        if template_name.contains("Glass") {
            ("ags".to_string(), "rofi".to_string(), "hyprpaper".to_string(), "hyprlock".to_string(), "ags".to_string())
        } else if template_name.contains("Nord") {
            ("waybar".to_string(), "rofi".to_string(), "hyprpaper".to_string(), "hyprlock".to_string(), "swaync".to_string())
        } else if template_name.contains("Cyber") {
            ("waybar".to_string(), "rofi".to_string(), "hyprpaper".to_string(), "hyprlock".to_string(), "swaync".to_string())
        } else {
            ("waybar".to_string(), "rofi".to_string(), "hyprpaper".to_string(), "hyprlock".to_string(), "swaync".to_string())
        }
    } else {
        let lower = prompt.to_lowercase();
        if lower.contains("blur") || lower.contains("glass") || lower.contains("widget") || lower.contains("glow") {
            ("ags".to_string(), "rofi".to_string(), "hyprpaper".to_string(), "hyprlock".to_string(), "ags".to_string())
        } else if lower.contains("minimal") || lower.contains("nord") || lower.contains("arctic") {
            ("waybar".to_string(), "rofi".to_string(), "hyprpaper".to_string(), "hyprlock".to_string(), "swaync".to_string())
        } else {
            ("waybar".to_string(), "rofi".to_string(), "hyprpaper".to_string(), "hyprlock".to_string(), "swaync".to_string())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptIntent {
    Edit,
    Redesign,
}

#[derive(Default, Debug, Clone)]
struct StackDetection {
    panel: Option<String>,
    launcher: Option<String>,
    wallpaper: Option<String>,
    lockscreen: Option<String>,
    notification: Option<String>,
    has_any: bool,
}

#[derive(Default, Debug, Clone)]
struct StackOverride {
    panel: Option<String>,
    launcher: Option<String>,
    wallpaper: Option<String>,
    lockscreen: Option<String>,
    notification: Option<String>,
}

fn prompt_suggests_redesign(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let keywords = [
        "redesign",
        "overhaul",
        "completely different",
        "from scratch",
        "fresh",
        "new theme",
        "replace everything",
        "full redesign",
    ];
    keywords.iter().any(|kw| lower.contains(kw))
}

fn prompt_mentions_keybinds(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let keywords = ["keybind", "key binding", "shortcut", "hotkey", "bind =", "binds"];
    keywords.iter().any(|kw| lower.contains(kw))
}

fn prompt_mentions_wallpaper(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let keywords = ["wallpaper", "background", "wall paper", "swww", "hyprpaper"];
    keywords.iter().any(|kw| lower.contains(kw))
}

fn prompt_mentions_lockscreen(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let keywords = ["lockscreen", "lock screen", "hyprlock", "swaylock", "waylock"];
    keywords.iter().any(|kw| lower.contains(kw))
}

fn classify_intent(prompt: &str, has_existing_setup: bool) -> PromptIntent {
    if prompt_suggests_redesign(prompt) {
        PromptIntent::Redesign
    } else if has_existing_setup {
        PromptIntent::Edit
    } else {
        PromptIntent::Redesign
    }
}

fn stack_override_from_prompt(prompt: &str) -> StackOverride {
    let lower = prompt.to_lowercase();
    let mut override_stack = StackOverride::default();

    if lower.contains("waybar") {
        override_stack.panel = Some("waybar".to_string());
    } else if lower.contains("ags") || lower.contains("aylur") {
        override_stack.panel = Some("ags".to_string());
    } else if lower.contains("quick-shell") || lower.contains("quickshell") {
        override_stack.panel = Some("quick-shell".to_string());
    } else if lower.contains("no panel") || lower.contains("without panel") {
        override_stack.panel = Some("none".to_string());
    }

    if lower.contains("rofi") {
        override_stack.launcher = Some("rofi".to_string());
    } else if lower.contains("no launcher") || lower.contains("without launcher") {
        override_stack.launcher = Some("none".to_string());
    }

    if lower.contains("hyprpaper") {
        override_stack.wallpaper = Some("hyprpaper".to_string());
    } else if lower.contains("no wallpaper") || lower.contains("without wallpaper") {
        override_stack.wallpaper = Some("none".to_string());
    }

    if lower.contains("hyprlock") {
        override_stack.lockscreen = Some("hyprlock".to_string());
    } else if lower.contains("swaylock") {
        override_stack.lockscreen = Some("swaylock".to_string());
    } else if lower.contains("waylock") {
        override_stack.lockscreen = Some("waylock".to_string());
    } else if lower.contains("no lockscreen") || lower.contains("without lockscreen") {
        override_stack.lockscreen = Some("none".to_string());
    }

    if lower.contains("swaync") {
        override_stack.notification = Some("swaync".to_string());
    } else if lower.contains("dunst") {
        override_stack.notification = Some("dunst".to_string());
    } else if lower.contains("no notification") || lower.contains("without notifications") {
        override_stack.notification = Some("none".to_string());
    }

    override_stack
}

fn detect_existing_stack(home_dir: &Path) -> StackDetection {
    let mut detection = StackDetection::default();
    let has = |path: &str| home_dir.join(path).exists();

    if has(".config/waybar") {
        detection.panel = Some("waybar".to_string());
        detection.has_any = true;
    } else if has(".config/ags") {
        detection.panel = Some("ags".to_string());
        detection.has_any = true;
    } else if has(".config/quickshell") || has(".config/quick-shell") {
        detection.panel = Some("quick-shell".to_string());
        detection.has_any = true;
    }

    if has(".config/rofi") {
        detection.launcher = Some("rofi".to_string());
        detection.has_any = true;
    }

    if has(".config/hypr/hyprpaper.conf") {
        detection.wallpaper = Some("hyprpaper".to_string());
        detection.has_any = true;
    }

    if has(".config/hypr/hyprlock.conf") {
        detection.lockscreen = Some("hyprlock".to_string());
        detection.has_any = true;
    } else if has(".config/swaylock/config") {
        detection.lockscreen = Some("swaylock".to_string());
        detection.has_any = true;
    } else if has(".config/waylock/config") {
        detection.lockscreen = Some("waylock".to_string());
        detection.has_any = true;
    }

    if has(".config/swaync") {
        detection.notification = Some("swaync".to_string());
        detection.has_any = true;
    } else if has(".config/dunst") {
        detection.notification = Some("dunst".to_string());
        detection.has_any = true;
    }

    detection
}

async fn perform_pre_run_backup(home_dir: &Path) -> Result<Option<PathBuf>, Box<dyn std::error::Error + Send + Sync>> {
    let hypr_config = home_dir.join(".config/hypr");
    if !hypr_config.exists() {
        return Ok(None);
    }

    println!("\n{}", warning("Existing Hyprland configuration detected at ~/.config/hypr."));
    print!("Would you like to back up your existing desktop configurations before proceeding? [Y/n]: ");
    use std::io::Write;
    std::io::stdout().flush()?;
    let mut choice_input = String::new();
    std::io::stdin().read_line(&mut choice_input)?;
    let choice = choice_input.trim().to_lowercase();
    if choice == "n" || choice == "no" {
        println!("{}", info("Skipping configuration backup."));
        return Ok(None);
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs()
        .to_string();
    let backup_root = home_dir.join(format!(".local/share/dotengine/backups/pre-run-{}", timestamp));
    tokio::fs::create_dir_all(&backup_root).await?;

    let folders = vec!["hypr", "waybar", "rofi", "dunst", "ags", "swaync", "quickshell", "quick-shell"];
    let mut backed_up = Vec::new();
    for folder in folders {
        let src = home_dir.join(".config").join(folder);
        if src.exists() {
            let dest = backup_root.join(folder);
            let status = tokio::process::Command::new("cp")
                .arg("-r")
                .arg(&src)
                .arg(&dest)
                .status()
                .await;
            if let Ok(st) = status {
                if st.success() {
                    backed_up.push(folder);
                }
            }
        }
    }

    if !backed_up.is_empty() {
        println!(
            "\n{}",
            success(&format!(
                "Successfully backed up active configuration directories {:?} to:\n    {}",
                backed_up,
                backup_root.display()
            ))
        );
        Ok(Some(backup_root))
    } else {
        Ok(None)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Enable logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    // 1. Initialize Ports & Infrastructure System Manager
    let system_manager: Arc<dyn SystemManager> = Arc::new(HyprSys::new());
    let home_dir = system_manager.get_home_directory();
    let existing_stack = detect_existing_stack(&home_dir);
    let has_existing_setup = existing_stack.has_any;
    let is_new_user = !has_existing_setup;
    let interactive = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();

    // Perform pre-run backup if an existing setup is detected
    let _ = perform_pre_run_backup(&system_manager.get_home_directory()).await;

    // 2. Load the selected AI provider's credential, prompting on its first use.
    let credentials = CredentialStore::new()?;
    let provider = credentials.select_provider(args.model.as_deref())?;
    println!("{} Using {}.", accent("Provider:"), provider.display_name());
    let api_key = credentials.get_or_prompt(provider)?;
    let ai_service: Arc<dyn AiService> = match provider {
        AiProvider::Openai => Arc::new(OpenaiClient::new(api_key)),
        AiProvider::Gemini => Arc::new(GeminiClient::new(api_key)),
    };

    // 3. Load Design Rules from design-skill.md & Technical Rules from skill.md (contained in project workspace root)
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let design_rules_path = workspace_root.join("design-skill.md");
    let skill_path = workspace_root.join("skill.md");

    let design_rules = if design_rules_path.exists() {
        std::fs::read_to_string(&design_rules_path)?
    } else {
        tracing::warn!("design-skill.md not found in project root. Falling back to default design specifications.");
        "Follow general visual balance guidelines, modern spacing, and rounded glassmorphic elements.".to_string()
    };

    let skill_content = if skill_path.exists() {
        std::fs::read_to_string(&skill_path)?
    } else {
        String::new()
    };

    let mut combined_rules = if skill_content.is_empty() {
        design_rules
    } else {
        format!(
            "{}\n\n=== TECHNICAL CONFIGURATION SYNTAX AND CAPABILITIES ===\n{}",
            design_rules, skill_content
        )
    };

    // 4. Handle Design Template Selection
    let use_composer = args.prompt.is_none();
    let selected_template = args.template;
    let mut raw_prompt = args.prompt;
    let mut image_payloads = Vec::new();

    if raw_prompt.is_none() {
        raw_prompt = Some(String::new());
    }

    let mut rofi_bind = args.rofi_bind;
    let wallpaper_util = args.wallpaper.clone();
    let lockscreen_util = args.lockscreen.clone();
    let notification_util = args.notification.clone();

    // 5. Load and base64-encode multimodal image if provided
    if let Some(img_path) = args.image {
        if img_path.exists() {
            println!(
                "{}",
                info(&format!("Loading design screenshot: {}...", img_path.display()))
            );
            let bytes = std::fs::read(&img_path)?;

            let encoded_image = ImagePayload::from_bytes(&bytes)?;
            println!(
                "{}",
                success(&format!(
                    "Screenshot encoded as {} ({} characters base64).",
                    encoded_image.media_type,
                    encoded_image.base64_data.len()
                ))
            );
            image_payloads.push(encoded_image);
        } else {
            return Err(format!(
                "Specified UI mockup image path does not exist: {}",
                img_path.display()
            )
            .into());
        }
    }

    let final_prompt = if use_composer {
        let composition = compose(
            raw_prompt.unwrap_or_else(|| "Nord minimalist desktop".to_string()),
            image_payloads,
            env!("CARGO_PKG_VERSION").to_string(),
            provider.display_name().to_string(),
        )?;
        image_payloads = composition.images;
        composition.instruction
    } else {
        raw_prompt.unwrap()
    };

    let system_context = activity(
        "Inspecting operating environment",
        system_manager.detect_system_context(),
    )
    .await?;
    println!(
        "{} Detected system context successfully.",
        accent("Dotengine")
    );
    println!("    Distro: {}", system_context.distribution);
    println!(
        "    Package Manager: {}",
        system_context
            .package_manager
            .as_deref()
            .unwrap_or("unsupported or unavailable")
    );
    println!("    Active Monitors: {}", system_context.monitors.len());
    for m in &system_context.monitors {
        println!(
            "      - {} ({}x{} @ {}Hz, scale: {})",
            m.name, m.width, m.height, m.refresh_rate, m.scale
        );
    }



    // 6. Determine intent and stack selection
    let intent = classify_intent(&final_prompt, has_existing_setup);
    let prompt_override = stack_override_from_prompt(&final_prompt);
    let (rec_panel, rec_launcher, rec_wallpaper, rec_lockscreen, rec_notification) =
        heuristic_stack_for_prompt(&final_prompt, selected_template);

    let panel_value = args
        .panel
        .clone()
        .or(prompt_override.panel)
        .or(existing_stack.panel)
        .unwrap_or(rec_panel);
    let launcher_value = args
        .launcher
        .clone()
        .or(prompt_override.launcher)
        .or(existing_stack.launcher)
        .unwrap_or(rec_launcher);
    let wallpaper_value = wallpaper_util
        .or(prompt_override.wallpaper)
        .or(existing_stack.wallpaper)
        .unwrap_or(rec_wallpaper);
    let lockscreen_value = lockscreen_util
        .or(prompt_override.lockscreen)
        .or(existing_stack.lockscreen)
        .unwrap_or(rec_lockscreen);
    let notification_value = notification_util
        .or(prompt_override.notification)
        .or(existing_stack.notification)
        .unwrap_or(rec_notification);
    let mut nm_applet_value = args.nm_applet;
    let mut blueman_value = args.blueman;

    if use_composer && is_new_user && args.panel.is_none() && args.launcher.is_none() {
        println!("\n{} Recommended desktop component stack for your design style:", accent("Dotengine"));
        println!("  - Panel/Shell:        {}", panel_value);
        println!("  - App Launcher:       {}", launcher_value);
        println!("  - Wallpaper Switcher: {}", wallpaper_value);
        println!("  - Lockscreen Tool:    {}", lockscreen_value);
        println!("  - Notification Panel: {}", notification_value);
        println!("  - Network Tray Icon:  {}", if nm_applet_value { "enabled" } else { "disabled (recommended: enable)" });
        println!("  - Bluetooth Tray Icon:{}", if blueman_value { "enabled" } else { "disabled (recommended: enable)" });
        nm_applet_value = true;
        blueman_value = true;
    } else if use_composer && !is_new_user {
        println!(
            "\n{} Detected existing desktop setup. Using current stack defaults unless your prompt overrides them.",
            accent("Dotengine")
        );
    }

    let wants_keybind_change = prompt_mentions_keybinds(&final_prompt);
    let should_prompt_rofi_bind = launcher_value == "rofi"
        && rofi_bind.is_none()
        && interactive
        && (is_new_user || intent == PromptIntent::Redesign || wants_keybind_change);

    if should_prompt_rofi_bind {
        print!("\nEnter preferred Rofi launch shortcut [default: SUPER, D]: ");
        use std::io::Write;
        std::io::stdout().flush()?;
        let mut rofi_input = String::new();
        std::io::stdin().read_line(&mut rofi_input)?;
        let trimmed = rofi_input.trim();
        rofi_bind = Some(if trimmed.is_empty() {
            "SUPER, D".to_string()
        } else {
            trimmed.to_string()
        });
    }
    let rofi_bind_value = if rofi_bind.is_some() {
        rofi_bind
    } else if launcher_value == "rofi" && is_new_user && interactive {
        Some("SUPER, D".to_string())
    } else {
        None
    };

    let mut wallpaper_prompt_val = args.wallpaper_prompt.clone().unwrap_or_default();
    let wants_wallpaper = prompt_mentions_wallpaper(&final_prompt);
    if wallpaper_prompt_val.is_empty()
        && wallpaper_value != "none"
        && use_composer
        && interactive
        && (is_new_user || wants_wallpaper)
    {
        print!("\nDescribe how the wallpaper should look (e.g. 'purple sunset minimalist mountains', or press enter for default): ");
        use std::io::Write;
        std::io::stdout().flush()?;
        let mut wall_prompt_input = String::new();
        std::io::stdin().read_line(&mut wall_prompt_input)?;
        wallpaper_prompt_val = wall_prompt_input.trim().to_string();
    }

    let mut lockscreen_prompt_val = args.lockscreen_prompt.clone().unwrap_or_default();
    let wants_lockscreen = prompt_mentions_lockscreen(&final_prompt);
    if lockscreen_prompt_val.is_empty()
        && lockscreen_value != "none"
        && use_composer
        && interactive
        && (is_new_user || wants_lockscreen)
    {
        print!("\nDescribe how the lockscreen should look (e.g. 'frosted glass centered login, analog clock', or press enter for default): ");
        use std::io::Write;
        std::io::stdout().flush()?;
        let mut lock_prompt_input = String::new();
        std::io::stdin().read_line(&mut lock_prompt_input)?;
        lockscreen_prompt_val = lock_prompt_input.trim().to_string();
    }

    // Hyprland version instructions
    let mut hyprland_version_note = String::new();
    if interactive {
        if let Some((version, uses_lua)) = system_context
            .hyprland_version
            .clone()
            .zip(system_context.hyprland_uses_lua)
        {
            hyprland_version_note = format!(
                "\nHYPRLAND VERSION DETECTED: {}. Use {} configuration syntax.",
                version,
                if uses_lua { "Lua (hyprland.lua)" } else { "Hyprlang (hyprland.conf)" }
            );
        } else {
            println!("\n{} Could not detect Hyprland version.", accent("Dotengine"));
            print!("Do you know your Hyprland version? (e.g. 0.55.1) [leave blank to use hyprlang]: ");
            use std::io::Write;
            std::io::stdout().flush()?;
            let mut version_input = String::new();
            std::io::stdin().read_line(&mut version_input)?;
            let version_input = version_input.trim();
            if !version_input.is_empty() {
                let uses_lua = version_input
                    .split('.')
                    .next()
                    .and_then(|major| major.parse::<u32>().ok())
                    .map(|major| major > 0)
                    .unwrap_or(false)
                    || version_input
                        .split('.')
                        .nth(1)
                        .and_then(|minor| minor.parse::<u32>().ok())
                        .map(|minor| minor >= 55)
                        .unwrap_or(false);
                hyprland_version_note = format!(
                    "\nHYPRLAND VERSION PROVIDED: {}. Use {} configuration syntax.",
                    version_input,
                    if uses_lua { "Lua (hyprland.lua)" } else { "Hyprlang (hyprland.conf)" }
                );
            } else {
                hyprland_version_note = "\nHYPRLAND VERSION UNKNOWN: default to Hyprlang (hyprland.conf) syntax.".to_string();
            }
        }
    }

    if !hyprland_version_note.is_empty() {
        combined_rules.push_str(&hyprland_version_note);
    }

    // 7. Construct and execute workflows (unlimited max self-healing retries)
    let healing_workflow = HealingWorkflow::new(ai_service.clone(), system_manager.clone(), usize::MAX);
    let generation_workflow =
        GenerationWorkflow::new(ai_service.clone(), system_manager.clone(), healing_workflow);

    println!(
        "\n{}",
        info("Starting dotfile generation pipeline...")
    );
    match generation_workflow
        .execute(
            final_prompt,
            image_payloads,
            selected_template,
            &combined_rules,
            rofi_bind_value.as_deref(),
            &panel_value,
            &launcher_value,
            &wallpaper_value,
            &lockscreen_value,
            &wallpaper_prompt_val,
            &lockscreen_prompt_val,
            &notification_value,
            nm_applet_value,
            blueman_value,
        )
        .await
    {
        Ok(configs) => {
            println!(
                "\n{}",
                heading("Dotengine completed configuration generation.")
            );
            println!("Generated files saved under your configuration directory:");
            for c in configs {
                println!("  - ~/{}", c.relative_path.display());
            }

            if wallpaper_value == "hyprpaper" {
                println!("\n{}", heading("HYPRPAPER WALLPAPER SWITCHER GUIDE"));
                println!("  To switch wallpapers dynamically using hyprpaper:");
                println!("  1. Preload a new wallpaper image into cache:");
                println!("     hyprctl hyprpaper preload \"/path/to/your/wallpaper.png\"");
                println!("  2. Apply the loaded wallpaper to your monitor:");
                println!("     hyprctl hyprpaper wallpaper \"monitor,/path/to/your/wallpaper.png\"");
                println!("  3. Add the preload and wallpaper lines to ~/.config/hypr/hyprpaper.conf to persist them.");
            }
        }
        Err(e) => {
            println!(
                "\n{}",
                error(&format!("Critical workflow failure: {}", e))
            );
            std::process::exit(1);
        }
    }

    Ok(())
}
