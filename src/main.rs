use clap::Parser;
use dotengine::application::{GenerationWorkflow, HealingWorkflow};
use dotengine::composer::compose;
use dotengine::domain::{DesignTemplate, ImagePayload};
use dotengine::infrastructure::{AiProvider, CredentialStore, GeminiClient, HyprSys, OpenaiClient};
use dotengine::ports::{AiService, SystemManager};
use dotengine::ui::{accent, heading, print_wordmark, success, info, warning, error};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "dotengine")]
#[command(author = "Aryan <aryan@dotengine.ai>")]
#[command(version = "1.0")]
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

    print_wordmark();

    let args = Args::parse();

    // 1. Initialize Ports & Infrastructure System Manager
    let system_manager: Arc<dyn SystemManager> = Arc::new(HyprSys::new());

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

    let combined_rules = if skill_content.is_empty() {
        design_rules
    } else {
        format!("{}\n\n=== TECHNICAL CONFIGURATION SYNTAX AND CAPABILITIES ===\n{}", design_rules, skill_content)
    };

    // 4. Handle Design Template Selection
    let predefined_templates = DesignTemplate::get_predefined_library();
    let use_composer = args.prompt.is_none();
    let mut selected_template = args.template;
    let mut raw_prompt = args.prompt;
    let mut image_payloads = Vec::new();

    if raw_prompt.is_none() && selected_template.is_none() {
        println!(
            "{} Choose a starting design profile:",
            heading("Welcome to Dotengine.")
        );
        for (i, t) in predefined_templates.iter().enumerate() {
            println!("  [{}] {} - {}", i, t.name, t.description);
        }
        println!(
            "  [{}] Custom Prompt (Define your own from scratch)",
            predefined_templates.len()
        );

        print!("\nEnter choice [0-{}]: ", predefined_templates.len());
        use std::io::Write;
        std::io::stdout().flush()?;

        let mut choice_input = String::new();
        std::io::stdin().read_line(&mut choice_input)?;
        let choice: usize = choice_input
            .trim()
            .parse()
            .unwrap_or(predefined_templates.len());

        if choice < predefined_templates.len() {
            selected_template = Some(choice);
            raw_prompt = Some(format!(
                "Apply layout settings matching {}",
                predefined_templates[choice].name
            ));
        } else {
            raw_prompt = Some(String::new());
        }
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
        )?;
        image_payloads = composition.images;
        composition.instruction
    } else {
        raw_prompt.unwrap()
    };

    // 6. Dynamic stack recommendation based on resolved prompt
    let (rec_panel, rec_launcher, rec_wallpaper, rec_lockscreen, rec_notification) = heuristic_stack_for_prompt(&final_prompt, selected_template);

    let mut panel_value = args.panel.clone().unwrap_or(rec_panel);
    let mut launcher_value = args.launcher.clone().unwrap_or(rec_launcher);
    let mut wallpaper_value = wallpaper_util.unwrap_or(rec_wallpaper);
    let mut lockscreen_value = lockscreen_util.unwrap_or(rec_lockscreen);
    let mut notification_value = notification_util.unwrap_or(rec_notification);
    let mut nm_applet_value = args.nm_applet;
    let mut blueman_value = args.blueman;

    if use_composer && args.panel.is_none() && args.launcher.is_none() && args.wallpaper.is_none() && args.lockscreen.is_none() && args.notification.is_none() {
        println!("\n{} Recommended desktop component stack for your design style:", accent("Dotengine"));
        println!("  - Panel/Shell:        {}", panel_value);
        println!("  - App Launcher:       {}", launcher_value);
        println!("  - Wallpaper Switcher: {}", wallpaper_value);
        println!("  - Lockscreen Tool:    {}", lockscreen_value);
        println!("  - Notification Panel: {}", notification_value);
        println!("  - Network Tray Icon:  {}", if nm_applet_value { "enabled" } else { "disabled (recommended: enable)" });
        println!("  - Bluetooth Tray Icon:{}", if blueman_value { "enabled" } else { "disabled (recommended: enable)" });

        print!("\nWould you like to customize this stack? [y/N]: ");
        use std::io::Write;
        std::io::stdout().flush()?;
        let mut cust_input = String::new();
        std::io::stdin().read_line(&mut cust_input)?;
        let choice = cust_input.trim().to_lowercase();
        if choice == "y" || choice == "yes" {
            // Customize Panel
            println!("\nChoose Panel/Shell:");
            println!("  [1] ags (recommended for Glassmorphic Glow)");
            println!("  [2] waybar (recommended for Nordic Minimalist)");
            println!("  [3] quick-shell");
            println!("  [4] none");
            print!("Enter choice [1-4, default: {}]: ", panel_value);
            std::io::stdout().flush()?;
            let mut panel_input = String::new();
            std::io::stdin().read_line(&mut panel_input)?;
            panel_value = match panel_input.trim() {
                "1" => "ags".to_string(),
                "2" => "waybar".to_string(),
                "3" => "quick-shell".to_string(),
                "4" => "none".to_string(),
                _ => panel_value,
            };

            // Customize Launcher
            println!("\nChoose App Launcher:");
            println!("  [1] rofi");
            println!("  [2] none");
            print!("Enter choice [1-2, default: {}]: ", launcher_value);
            std::io::stdout().flush()?;
            let mut launcher_input = String::new();
            std::io::stdin().read_line(&mut launcher_input)?;
            launcher_value = match launcher_input.trim() {
                "1" => "rofi".to_string(),
                "2" => "none".to_string(),
                _ => launcher_value,
            };

            // Customize Wallpaper
            println!("\nChoose Wallpaper utility:");
            println!("  [1] hyprpaper");
            println!("  [2] none");
            print!("Enter choice [1-2, default: {}]: ", wallpaper_value);
            std::io::stdout().flush()?;
            let mut wallpaper_input = String::new();
            std::io::stdin().read_line(&mut wallpaper_input)?;
            wallpaper_value = match wallpaper_input.trim() {
                "1" => "hyprpaper".to_string(),
                "2" => "none".to_string(),
                _ => wallpaper_value,
            };

            // Customize Lockscreen
            println!("\nChoose Lockscreen utility:");
            println!("  [1] hyprlock");
            println!("  [2] swaylock");
            println!("  [3] waylock");
            println!("  [4] none");
            print!("Enter choice [1-4, default: {}]: ", lockscreen_value);
            std::io::stdout().flush()?;
            let mut lockscreen_input = String::new();
            std::io::stdin().read_line(&mut lockscreen_input)?;
            lockscreen_value = match lockscreen_input.trim() {
                "1" => "hyprlock".to_string(),
                "2" => "swaylock".to_string(),
                "3" => "waylock".to_string(),
                "4" => "none".to_string(),
                _ => lockscreen_value,
            };

            // Customize Notification Center
            println!("\nChoose Notification Center:");
            println!("  [1] swaync (Sway Notification Center)");
            println!("  [2] dunst");
            println!("  [3] none");
            print!("Enter choice [1-3, default: {}]: ", notification_value);
            std::io::stdout().flush()?;
            let mut notif_input = String::new();
            std::io::stdin().read_line(&mut notif_input)?;
            notification_value = match notif_input.trim() {
                "1" => "swaync".to_string(),
                "2" => "dunst".to_string(),
                "3" => "none".to_string(),
                _ => notification_value,
            };

            // Customize Applets
            print!("\nEnable Network Manager tray applet (nm-applet)? [Y/n]: ");
            std::io::stdout().flush()?;
            let mut nm_input = String::new();
            std::io::stdin().read_line(&mut nm_input)?;
            nm_applet_value = !matches!(nm_input.trim().to_lowercase().as_str(), "n" | "no");

            print!("Enable Bluetooth connection tray applet (blueman)? [Y/n]: ");
            std::io::stdout().flush()?;
            let mut blue_input = String::new();
            std::io::stdin().read_line(&mut blue_input)?;
            blueman_value = !matches!(blue_input.trim().to_lowercase().as_str(), "n" | "no");
        } else {
            // Suggest enabling applets if they are currently disabled in default non-interactive
            nm_applet_value = true;
            blueman_value = true;
        }
    }

    if rofi_bind.is_none() && launcher_value == "rofi" && use_composer {
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
    let rofi_bind_value = rofi_bind.unwrap_or_else(|| "SUPER, D".to_string());

    let mut wallpaper_prompt_val = args.wallpaper_prompt.clone().unwrap_or_default();
    if wallpaper_prompt_val.is_empty() && wallpaper_value != "none" && use_composer {
        print!("\nDescribe how the wallpaper should look (e.g. 'purple sunset minimalist mountains', or press enter for default): ");
        use std::io::Write;
        std::io::stdout().flush()?;
        let mut wall_prompt_input = String::new();
        std::io::stdin().read_line(&mut wall_prompt_input)?;
        wallpaper_prompt_val = wall_prompt_input.trim().to_string();
    }

    let mut lockscreen_prompt_val = args.lockscreen_prompt.clone().unwrap_or_default();
    if lockscreen_prompt_val.is_empty() && lockscreen_value != "none" && use_composer {
        print!("\nDescribe how the lockscreen should look (e.g. 'frosted glass centered login, analog clock', or press enter for default): ");
        use std::io::Write;
        std::io::stdout().flush()?;
        let mut lock_prompt_input = String::new();
        std::io::stdin().read_line(&mut lock_prompt_input)?;
        lockscreen_prompt_val = lock_prompt_input.trim().to_string();
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
            &rofi_bind_value,
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
