use clap::{ArgAction, Parser};
use dotengine::application::{
    archetype_guidelines, audit_desktop_completeness,
    cli_plan::{
        classify_intent, detect_existing_stack, heuristic_stack_for_prompt,
        prompt_mentions_lockscreen, prompt_mentions_wallpaper, should_prompt_rofi_bind,
        stack_override_from_prompt, validate_template_index, BackupMode, ComponentStack, RunPlan,
    },
    normalize_stack_values,
    skill_corpus::SkillCorpus,
    GenerationWorkflow, HealingWorkflow,
};
use dotengine::composer::compose;
use dotengine::domain::{DesignReferenceSpec, DesignTemplate, ImagePayload, UserPrompt};
use dotengine::infrastructure::{AiProvider, CredentialStore, GeminiClient, HyprSys, OpenaiClient};
use dotengine::ports::{AiService, SystemManager};
use dotengine::ui::{accent, error, heading, info, success, warning};
use std::io::{IsTerminal, Write};
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

    /// Preview the resolved plan without making any changes.
    #[arg(long)]
    dry_run: bool,

    /// Refuse to prompt for interactive input and fail fast when a value is missing.
    #[arg(long)]
    non_interactive: bool,

    /// Backup policy for existing Hyprland configs before applying changes.
    #[arg(long, value_enum, default_value_t = BackupMode::Prompt)]
    backup_mode: BackupMode,

    /// Disable automatic package installation attempts for missing optional dependencies.
    #[arg(long = "no-auto-install", action = ArgAction::SetTrue)]
    no_auto_install: bool,

    /// Refresh the cached upstream Hyprland docs into the local user cache before generation.
    #[arg(long)]
    refresh_skills: bool,

    /// Change preferred AI provider preference interactively and exit
    #[arg(long)]
    change_provider: bool,

    /// Change/update the API key for the selected AI provider (specified by --model or default preferred) and exit
    #[arg(long)]
    change_key: bool,
}


fn merge_stack_choice(
    cli_choice: Option<String>,
    prompt_choice: &str,
    existing_choice: Option<String>,
    heuristic_choice: &str,
) -> String {
    cli_choice
        .or_else(|| {
            if prompt_choice.is_empty() {
                None
            } else {
                Some(prompt_choice.to_string())
            }
        })
        .or(existing_choice)
        .unwrap_or_else(|| heuristic_choice.to_string())
}

async fn load_image_payloads(
    image: Option<PathBuf>,
) -> Result<Vec<ImagePayload>, Box<dyn std::error::Error + Send + Sync>> {
    let mut image_payloads = Vec::new();

    if let Some(img_path) = image {
        if !img_path.exists() {
            return Err(format!(
                "Specified UI mockup image path does not exist: {}",
                img_path.display()
            )
            .into());
        }

        println!(
            "{}",
            info(&format!(
                "Loading design screenshot: {}...",
                img_path.display()
            ))
        );
        let bytes = std::fs::read(&img_path)?;
        let encoded_image = ImagePayload::from_reference_image_bytes(&bytes)?;
        println!(
            "{}",
            success(&format!(
                "Screenshot encoded as {} ({} characters base64).",
                encoded_image.media_type,
                encoded_image.base64_data.len()
            ))
        );
        image_payloads.push(encoded_image);
    }

    Ok(image_payloads)
}

fn load_design_rules() -> Result<SkillCorpus, Box<dyn std::error::Error + Send + Sync>> {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    SkillCorpus::load(workspace_root)
}

fn load_hyprland_config_text(home_dir: &Path) -> Option<String> {
    let candidates = [
        home_dir.join(".config/hypr/hyprland.conf"),
        home_dir.join(".config/hypr/hyprland.lua"),
    ];

    for path in candidates {
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if !contents.trim().is_empty() {
                return Some(contents);
            }
        }
    }

    None
}

fn preview_plan(
    plan: &RunPlan,
    provider_label: &str,
    final_prompt: &str,
    template_name: Option<&str>,
    image_count: usize,
) {
    println!("\n{}", heading("Dotengine dry-run preview"));
    println!("  Provider: {}", provider_label);
    println!("  Interactive: {}", plan.interactive);
    println!("  Non-interactive: {}", plan.non_interactive);
    println!("  Backup mode: {:?}", plan.backup_mode);
    println!(
        "  Auto install optional deps: {}",
        if plan.auto_install {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!("  Template index: {:?}", plan.template_index);
    if let Some(name) = template_name {
        println!("  Template name: {}", name);
    }
    println!("  Prompt intent: {:?}", plan.prompt_intent);
    println!("  Attached images: {}", image_count);
    println!("  Final prompt: {}", final_prompt);
    println!("  Panel: {}", plan.stack.panel);
    println!("  Launcher: {}", plan.stack.launcher);
    println!("  Wallpaper: {}", plan.stack.wallpaper);
    println!("  Lockscreen: {}", plan.stack.lockscreen);
    println!("  Notifications: {}", plan.stack.notification);
}

async fn perform_pre_run_backup(
    home_dir: &Path,
    backup_mode: BackupMode,
    interactive: bool,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error + Send + Sync>> {
    let hypr_config = home_dir.join(".config/hypr");
    if !hypr_config.exists() || matches!(backup_mode, BackupMode::Off) {
        return Ok(None);
    }

    if matches!(backup_mode, BackupMode::Prompt) && !interactive {
        println!(
            "{} Backup prompt suppressed because the CLI is running non-interactively.",
            warning("Dotengine")
        );
        return Ok(None);
    }

    if matches!(backup_mode, BackupMode::Prompt) {
        println!(
            "\n{}",
            warning("Existing Hyprland configuration detected at ~/.config/hypr.")
        );
        print!(
            "Would you like to back up your existing desktop configurations before proceeding? [Y/n]: "
        );
        std::io::stdout().flush()?;
        let mut choice_input = String::new();
        std::io::stdin().read_line(&mut choice_input)?;
        let choice = choice_input.trim().to_lowercase();
        if choice == "n" || choice == "no" {
            println!("{}", info("Skipping configuration backup."));
            return Ok(None);
        }
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs()
        .to_string();
    let backup_root = home_dir.join(format!(
        ".local/share/dotengine/backups/pre-run-{}",
        timestamp
    ));
    tokio::fs::create_dir_all(&backup_root).await?;

    let folders = vec![
        "hypr",
        "waybar",
        "rofi",
        "dunst",
        "ags",
        "swaync",
        "quickshell",
        "quick-shell",
    ];
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
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    let interactive =
        !args.non_interactive && std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    let auto_install_optional_deps = interactive && !args.no_auto_install;

    let system_manager: Arc<dyn SystemManager> = Arc::new(HyprSys::new());
    let home_dir = system_manager.get_home_directory();
    let existing_stack = detect_existing_stack(&home_dir);
    let is_new_user = !existing_stack.has_any;

    let template_index = validate_template_index(args.template)?;

    let provider_label = args
        .model
        .as_deref()
        .map(|model| {
            AiProvider::from_model_name(model).map(|provider| provider.display_name().to_string())
        })
        .transpose()?
        .unwrap_or_else(|| "saved provider".to_string());

    let mut image_payloads = load_image_payloads(args.image.clone()).await?;

    let mut raw_prompt = match args.prompt.clone() {
        Some(prompt) => prompt,
        None if interactive => {
            let composition = compose(
                String::new(),
                image_payloads,
                env!("CARGO_PKG_VERSION").to_string(),
                provider_label.clone(),
            )?;
            image_payloads = composition.images;
            composition.instruction
        }
        None => {
            return Err(
                "A prompt is required when running non-interactively. Pass --prompt or enable a terminal session."
                    .into(),
            );
        }
    };

    let prompt_intent = classify_intent(&raw_prompt, existing_stack.has_any);
    let prompt_override = stack_override_from_prompt(&raw_prompt);
    let heuristic_stack = heuristic_stack_for_prompt(&raw_prompt, template_index)?;

    let mut panel_value = merge_stack_choice(
        args.panel.clone(),
        if prompt_override.panel.is_empty() {
            ""
        } else {
            &prompt_override.panel
        },
        existing_stack.panel.clone(),
        &heuristic_stack.panel,
    );
    let mut launcher_value = merge_stack_choice(
        args.launcher.clone(),
        if prompt_override.launcher.is_empty() {
            ""
        } else {
            &prompt_override.launcher
        },
        existing_stack.launcher.clone(),
        &heuristic_stack.launcher,
    );
    let mut wallpaper_value = merge_stack_choice(
        args.wallpaper.clone(),
        if prompt_override.wallpaper.is_empty() {
            ""
        } else {
            &prompt_override.wallpaper
        },
        existing_stack.wallpaper.clone(),
        &heuristic_stack.wallpaper,
    );
    let mut lockscreen_value = merge_stack_choice(
        args.lockscreen.clone(),
        if prompt_override.lockscreen.is_empty() {
            ""
        } else {
            &prompt_override.lockscreen
        },
        existing_stack.lockscreen.clone(),
        &heuristic_stack.lockscreen,
    );
    let mut notification_value = merge_stack_choice(
        args.notification.clone(),
        if prompt_override.notification.is_empty() {
            ""
        } else {
            &prompt_override.notification
        },
        existing_stack.notification.clone(),
        &heuristic_stack.notification,
    );

    let dry_run_plan = RunPlan {
        prompt_intent,
        stack: ComponentStack::new(
            panel_value.clone(),
            launcher_value.clone(),
            wallpaper_value.clone(),
            lockscreen_value.clone(),
            notification_value.clone(),
        ),
        template_index,
        template_name: {
            let templates = DesignTemplate::get_predefined_library();
            template_index
                .and_then(|idx| templates.get(idx))
                .map(|template| template.name.clone())
        },
        interactive,
        dry_run: args.dry_run,
        non_interactive: args.non_interactive,
        backup_mode: args.backup_mode,
        auto_install: auto_install_optional_deps,
    };

    if dry_run_plan.dry_run {
        preview_plan(
            &dry_run_plan,
            &provider_label,
            &raw_prompt,
            dry_run_plan.template_name.as_deref(),
            image_payloads.len(),
        );
        if !image_payloads.is_empty() {
            println!(
                "{} Reference-image analysis is skipped in dry-run mode.",
                warning("Dotengine")
            );
        }
        return Ok(());
    }

    let credentials = CredentialStore::new()?;

    if args.change_provider {
        let chosen = if let Some(requested) = args.model.as_deref() {
            let p = AiProvider::from_model_name(requested)?;
            credentials.change_preferred_provider(p)?;
            p
        } else {
            credentials.prompt_and_save_provider()?
        };
        println!("{} Successfully changed preferred default AI provider to {}.", accent("Dotengine"), chosen.display_name());

        if !credentials.has_key(chosen)? {
            println!("{} No stored API key found for {}. Setup is required.", accent("Dotengine"), chosen.display_name());
            credentials.prompt_and_save_key(chosen)?;
            println!("{} Successfully saved the API key for {}.", accent("Dotengine"), chosen.display_name());
        }
        return Ok(());
    }

    if args.change_key {
        let target_provider = if let Some(requested) = args.model.as_deref() {
            AiProvider::from_model_name(requested)?
        } else {
            credentials.select_provider(None, true)?
        };
        credentials.prompt_and_save_key(target_provider)?;
        println!("{} Successfully updated and saved the API key for {}.", accent("Dotengine"), target_provider.display_name());
        return Ok(());
    }

    let provider = credentials.select_provider(args.model.as_deref(), interactive)?;
    println!("{} Using {}.", accent("Provider:"), provider.display_name());
    let api_key = credentials.get_or_prompt(provider, interactive)?;
    let ai_service: Arc<dyn AiService> = match provider {
        AiProvider::Openai => Arc::new(OpenaiClient::new(api_key)),
        AiProvider::Gemini => Arc::new(GeminiClient::new(api_key)),
    };

    if args.refresh_skills && !args.dry_run {
        match SkillCorpus::refresh_upstream_cache().await {
            Ok(_) => println!(
                "{} Refreshed cached upstream Hypr docs for the skill corpus.",
                accent("Dotengine")
            ),
            Err(error) => println!(
                "{} Could not refresh upstream Hypr docs; continuing with the local corpus: {}",
                warning("Dotengine"),
                error
            ),
        }
    } else if args.refresh_skills {
        println!(
            "{} Skipping upstream skill refresh in dry-run mode.",
            warning("Dotengine")
        );
    }

    let skill_corpus = load_design_rules()?;
    let mut combined_rules = skill_corpus.combined();
    let reference_analysis_rules = skill_corpus.reference_analysis_guidance();

    let predefined_templates = DesignTemplate::get_predefined_library();
    let archetype_template_name = template_index
        .and_then(|idx| predefined_templates.get(idx))
        .map(|template| template.name.as_str());
    if let Some(archetype_rules) = archetype_guidelines(&raw_prompt, archetype_template_name) {
        combined_rules.push_str("\n\n=== DESIGN ARCHETYPE GUIDANCE ===\n");
        combined_rules.push_str(&archetype_rules);
    }

    let system_context = system_manager.detect_system_context().await?;
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

    let mut reference_spec: Option<DesignReferenceSpec> = None;
    if !image_payloads.is_empty() {
        println!(
            "{} Analyzing attached reference design screenshots...",
            accent("Dotengine")
        );
        let reference_prompt =
            UserPrompt::new(raw_prompt.clone()).with_images(image_payloads.clone());
        match ai_service
            .analyze_design_reference(
                &reference_prompt,
                &system_context,
                &reference_analysis_rules,
            )
            .await
        {
            Ok(analyzed) => {
                if analyzed.summary.trim().is_empty() {
                    println!(
                        "{} Reference analysis returned no summary. Falling back to prompt-driven generation.",
                        warning("Dotengine")
                    );
                } else {
                    println!(
                        "{} Reference design summary: {}",
                        accent("Dotengine"),
                        analyzed.summary
                    );
                    if let Some(palette) = &analyzed.palette {
                        println!("    Palette: {}", palette);
                    }
                    if !analyzed.startup_commands.is_empty() {
                        println!("    Startup wiring inferred:");
                        for command in &analyzed.startup_commands {
                            println!("      - {}", command);
                        }
                    }
                    reference_spec = Some(analyzed);
                }
            }
            Err(error) => {
                println!(
                    "{} Reference analysis failed; continuing with prompt-driven generation: {}",
                    warning("Dotengine"),
                    error
                );
            }
        }
        if raw_prompt.trim().is_empty() {
            raw_prompt =
                "Recreate the attached reference desktop setup with a complete Hyprland rice."
                    .to_string();
        }
    }

    if let Some(reference) = reference_spec.as_ref() {
        let (panel, launcher, wallpaper, lockscreen, notification) =
            reference.stack.apply_to_values(
                panel_value.clone(),
                launcher_value.clone(),
                wallpaper_value.clone(),
                lockscreen_value.clone(),
                notification_value.clone(),
            );
        panel_value = panel;
        launcher_value = launcher;
        wallpaper_value = wallpaper;
        lockscreen_value = lockscreen;
        notification_value = notification;
    }

    let (panel_value, launcher_value, wallpaper_value, lockscreen_value, notification_value) =
        normalize_stack_values(
            panel_value,
            launcher_value,
            wallpaper_value,
            lockscreen_value,
            notification_value,
        );

    let resolved_stack = ComponentStack::new(
        panel_value.clone(),
        launcher_value.clone(),
        wallpaper_value.clone(),
        lockscreen_value.clone(),
        notification_value.clone(),
    );
    let completeness_report = audit_desktop_completeness(
        &resolved_stack,
        &existing_stack,
        &system_context,
        load_hyprland_config_text(&home_dir).as_deref(),
    );
    if !completeness_report.is_clean() {
        println!("\n{}", warning("Desktop completeness audit"));
        for issue in &completeness_report.issues {
            let severity = match issue.severity {
                dotengine::application::desktop_audit::AuditSeverity::Info => "info",
                dotengine::application::desktop_audit::AuditSeverity::Warning => "warning",
            };
            println!("  - [{}] {}: {}", severity, issue.component, issue.detail);
        }
    }

    let run_plan = RunPlan {
        prompt_intent,
        stack: resolved_stack,
        template_index,
        template_name: {
            let templates = DesignTemplate::get_predefined_library();
            template_index
                .and_then(|idx| templates.get(idx))
                .map(|template| template.name.clone())
        },
        interactive,
        dry_run: args.dry_run,
        non_interactive: args.non_interactive,
        backup_mode: args.backup_mode,
        auto_install: auto_install_optional_deps,
    };

    let backup_result = perform_pre_run_backup(&home_dir, args.backup_mode, interactive).await?;
    if let Some(path) = backup_result {
        println!(
            "{} Created backup archive at {} before applying changes.",
            accent("Dotengine"),
            path.display()
        );
    }

    let should_prompt_rofi_bind = should_prompt_rofi_bind(
        &launcher_value,
        args.rofi_bind.is_none(),
        interactive,
        is_new_user,
        run_plan.prompt_intent,
        &raw_prompt,
    );

    let mut rofi_bind = args.rofi_bind.clone();
    if should_prompt_rofi_bind {
        print!("\nEnter preferred Rofi launch shortcut [default: SUPER, D]: ");
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
    if wallpaper_prompt_val.is_empty()
        && wallpaper_value != "none"
        && interactive
        && (is_new_user || prompt_mentions_wallpaper(&raw_prompt))
    {
        print!(
            "\nDescribe how the wallpaper should look (e.g. 'purple sunset minimalist mountains', or press enter for default): "
        );
        std::io::stdout().flush()?;
        let mut wall_prompt_input = String::new();
        std::io::stdin().read_line(&mut wall_prompt_input)?;
        wallpaper_prompt_val = wall_prompt_input.trim().to_string();
    }

    let mut lockscreen_prompt_val = args.lockscreen_prompt.clone().unwrap_or_default();
    if lockscreen_prompt_val.is_empty()
        && lockscreen_value != "none"
        && interactive
        && (is_new_user || prompt_mentions_lockscreen(&raw_prompt))
    {
        print!(
            "\nDescribe how the lockscreen should look (e.g. 'frosted glass centered login, analog clock', or press enter for default): "
        );
        std::io::stdout().flush()?;
        let mut lock_prompt_input = String::new();
        std::io::stdin().read_line(&mut lock_prompt_input)?;
        lockscreen_prompt_val = lock_prompt_input.trim().to_string();
    }

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
                if uses_lua {
                    "Lua (hyprland.lua)"
                } else {
                    "Hyprlang (hyprland.conf)"
                }
            );
        } else {
            println!(
                "\n{} Could not detect Hyprland version.",
                accent("Dotengine")
            );
            print!(
                "Do you know your Hyprland version? (e.g. 0.55.1) [leave blank to use hyprlang]: "
            );
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
                    if uses_lua {
                        "Lua (hyprland.lua)"
                    } else {
                        "Hyprlang (hyprland.conf)"
                    }
                );
            } else {
                hyprland_version_note =
                    "\nHYPRLAND VERSION UNKNOWN: default to Hyprlang (hyprland.conf) syntax."
                        .to_string();
            }
        }
    }

    if !hyprland_version_note.is_empty() {
        combined_rules.push_str(&hyprland_version_note);
    }

    let healing_workflow =
        HealingWorkflow::new(ai_service.clone(), system_manager.clone(), 5);
    let generation_workflow =
        GenerationWorkflow::new(ai_service.clone(), system_manager.clone(), healing_workflow);

    println!("\n{}", info("Starting dotfile generation pipeline..."));
    match generation_workflow
        .execute(
            raw_prompt,
            image_payloads,
            template_index,
            &combined_rules,
            reference_spec.as_ref(),
            rofi_bind_value.as_deref(),
            &panel_value,
            &launcher_value,
            &wallpaper_value,
            &lockscreen_value,
            &wallpaper_prompt_val,
            &lockscreen_prompt_val,
            &notification_value,
            args.nm_applet,
            args.blueman,
            auto_install_optional_deps,
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
                println!(
                    "     hyprctl hyprpaper wallpaper \"monitor,/path/to/your/wallpaper.png\""
                );
                println!("  3. Add the preload and wallpaper lines to ~/.config/hypr/hyprpaper.conf to persist them.");
            }
        }
        Err(e) => {
            println!("\n{}", error(&format!("Critical workflow failure: {}", e)));
            std::process::exit(1);
        }
    }

    Ok(())
}
