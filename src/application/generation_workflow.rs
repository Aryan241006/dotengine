use crate::application::{
    archetype_guidelines, archetype_recommendation_lines, healing_workflow::HealingWorkflow,
};
use crate::domain::{ConfigFile, DesignReferenceSpec, DesignTemplate, ImagePayload, UserPrompt};
use crate::ports::{AiService, SystemManager};
use crate::ui::{accent, activity, heading};
use std::sync::Arc;

pub struct GenerationWorkflow {
    ai_service: Arc<dyn AiService>,
    system_manager: Arc<dyn SystemManager>,
    healing_workflow: HealingWorkflow,
}

impl GenerationWorkflow {
    pub fn new(
        ai_service: Arc<dyn AiService>,
        system_manager: Arc<dyn SystemManager>,
        healing_workflow: HealingWorkflow,
    ) -> Self {
        Self {
            ai_service,
            system_manager,
            healing_workflow,
        }
    }

    pub fn recommend_software(&self, prompt_text: &str, chosen_template: Option<&DesignTemplate>) {
        println!("\n{}", heading("Dotengine software recommendations"));

        if let Some(template) = chosen_template {
            println!("Selected Design Profile: {}", template.name);
            println!("Recommended Stack: {:?}", template.recommended_stack);
            println!("\nRecommendation:");
            if let Some(lines) = archetype_recommendation_lines(prompt_text, Some(&template.name)) {
                for line in lines {
                    println!("{}", line);
                }
            } else {
                for item in &template.recommended_stack {
                    println!("- [RECOMMENDED] {}", item);
                }
            }
        } else {
            if let Some(lines) = archetype_recommendation_lines(prompt_text, None) {
                for line in lines {
                    println!("{}", line);
                }
            } else {
                println!("Target Aesthetic: General custom.");
                println!("- [RECOMMENDED] Waybar & rofi: Traditional highly configurable Hyprland workspace combo.");
            }
        }
        println!();
    }

    pub async fn execute(
        &self,
        raw_prompt: String,
        image_payloads: Vec<ImagePayload>,
        selected_template_index: Option<usize>,
        design_rules_content: &str,
        reference_spec: Option<&DesignReferenceSpec>,
        rofi_bind: Option<&str>,
        panel_util: &str,
        launcher_util: &str,
        wallpaper_util: &str,
        lockscreen_util: &str,
        wallpaper_prompt: &str,
        lockscreen_prompt: &str,
        notification_util: &str,
        enable_nm_applet: bool,
        enable_blueman: bool,
        auto_install_optional_deps: bool,
    ) -> Result<Vec<ConfigFile>, Box<dyn std::error::Error + Send + Sync>> {
        // Intercept Direct Actions (e.g. Set Wallpaper)
        let prompt_intent = crate::application::cli_plan::classify_intent(&raw_prompt, true);
        if prompt_intent == crate::application::cli_plan::PromptIntent::Action
            && crate::application::cli_plan::prompt_is_direct_action(&raw_prompt) {
                if let Some(extracted_path) = extract_path_from_prompt(&raw_prompt) {
                    println!("\n{} Executing direct action: Set wallpaper to '{}'", accent("Dotengine"), extracted_path);
                    let normalized = normalize_path(&extracted_path);
                    
                    let content = format!(
                        "preload = {}\nwallpaper {{\n    monitor = \n    path = {}\n    fit_mode = cover\n}}\n",
                        normalized.display(),
                        normalized.display()
                    );
                    
                    let config_file = ConfigFile {
                        relative_path: std::path::PathBuf::from(".config/hypr/hyprpaper.conf"),
                        content: content.clone(),
                    };
                    
                    self.system_manager.write_config_file(&config_file).await?;
                    
                    self.system_manager.verify_and_reload(
                        std::slice::from_ref(&config_file),
                        normalized.to_str(),
                    ).await
                    .map_err(|e| Box::new(std::io::Error::other(format!("{:?}", e))) as Box<dyn std::error::Error + Send + Sync>)?;
                    
                    println!("{} Wallpaper change successfully completed.", accent("Dotengine"));
                    return Ok(vec![config_file]);
                }
            }

        let predefined = DesignTemplate::get_predefined_library();
        let mut custom_guidelines = None;
        let mut chosen_template = None;

        if let Some(idx) = selected_template_index {
            if idx < predefined.len() {
                let template = &predefined[idx];
                chosen_template = Some(template);
                println!(
                    "{} Seeded workflow using profile: {}",
                    accent("Dotengine"),
                    template.name
                );
                custom_guidelines = Some(format!(
                    "Aesthetic Profile: {}\nRounding: {}px\nGaps-In: {}px\nGaps-Out: {}px\nAnimations: {}",
                    template.name,
                    template.recommended_rounding,
                    template.recommended_gaps_in,
                    template.recommended_gaps_out,
                    if template.dynamic_animations { "Springy / Fluid (wind curve)" } else { "Classic" }
                ));
                if let Some(archetype) = archetype_guidelines(&template.name, Some(&template.name))
                {
                    if let Some(current) = custom_guidelines.take() {
                        custom_guidelines = Some(format!("{}\n{}", current, archetype));
                    } else {
                        custom_guidelines = Some(archetype);
                    }
                }
            }
        }

        if let Some(reference) = reference_spec {
            println!(
                "{} Reference design extracted from screenshot: {}",
                accent("Dotengine"),
                reference.summary
            );
            if !reference.startup_commands.is_empty() {
                println!("{} Reference startup wiring:", accent("Dotengine"));
                for command in &reference.startup_commands {
                    println!("    - {}", command);
                }
            }
        }

        // Recommend software stack based on preferences
        self.recommend_software(&raw_prompt, chosen_template);

        // Scan system parameters
        let system_context = self.system_manager.detect_system_context().await?;

        // Dynamically build check list from selected components
        let mut tools_to_check = Vec::new();
        if panel_util != "none" {
            tools_to_check.push(panel_util.to_string());
        }
        if launcher_util != "none" {
            tools_to_check.push(launcher_util.to_string());
        }
        if wallpaper_util != "none" {
            tools_to_check.push(wallpaper_util.to_string());
        }
        if lockscreen_util != "none" {
            tools_to_check.push(lockscreen_util.to_string());
            tools_to_check.push("hypridle".to_string());
            tools_to_check.push("wlogout".to_string());
        }
        if notification_util != "none" {
            tools_to_check.push(notification_util.to_string());
        }
        if enable_nm_applet {
            tools_to_check.push("network-manager-applet".to_string());
        }
        if enable_blueman {
            tools_to_check.push("blueman".to_string());
        }

        for tool in tools_to_check {
            // Translate applet commands to check presence
            let bin_name = match tool.as_str() {
                "network-manager-applet" => "nm-applet",
                "blueman" => "blueman-applet",
                other => other,
            };
            let installed = self.system_manager.check_command_installed(bin_name).await;
            if !installed {
                println!(
                    "{} Recommended component '{}' is not installed in the system PATH.",
                    accent("Dotengine"),
                    tool
                );
                if auto_install_optional_deps {
                    if let Err(e) = self.system_manager.install_package(&tool).await {
                        println!(
                            "{} Could not install optional dependency '{}': {}. Continuing without it.",
                            crate::ui::warning("Dotengine"),
                            tool,
                            e
                        );
                    }
                } else {
                    println!(
                        "{} Skipping optional install for '{}'. Continue with the current desktop stack or install it manually later.",
                        crate::ui::warning("Dotengine"),
                        tool
                    );
                }
            } else {
                println!(
                    "{} Verified prerequisite '{}' is installed.",
                    accent("Dotengine"),
                    tool
                );
            }
        }

        // Check Font Awesome installation in system font cache
        let font_installed = system_context
            .package_status
            .get("fonts-font-awesome")
            .copied()
            .unwrap_or(false);
        if !font_installed {
            println!(
                "{} Required icon glyph font 'FontAwesome' is missing from the system font cache.",
                accent("Dotengine")
            );
            if auto_install_optional_deps {
                if let Err(e) = self
                    .system_manager
                    .install_package("fonts-font-awesome")
                    .await
                {
                    println!(
                        "{} Could not install FontAwesome automatically: {}. Continuing with fallback glyphs.",
                        crate::ui::warning("Dotengine"),
                        e
                    );
                }
            } else {
                println!(
                    "{} FontAwesome is unavailable. Continuing with fallback glyphs and lower-fidelity icons.",
                    crate::ui::warning("Dotengine")
                );
            }
        } else {
            println!(
                "{} Verified prerequisite 'FontAwesome' is installed.",
                accent("Dotengine")
            );
        }

        // Check Nerd Font installation in system font cache
        let nerd_font_installed = system_context
            .package_status
            .get("fonts-nerd-font")
            .copied()
            .unwrap_or(false);
        if !nerd_font_installed {
            println!(
                "{} Required icon glyph font 'JetBrainsMono Nerd Font' is missing from the system font cache.",
                accent("Dotengine")
            );
            println!(
                "{} JetBrainsMono Nerd Font is optional. Continuing without network downloads; icon coverage may degrade in terminal previews.",
                crate::ui::warning("Dotengine")
            );
        } else {
            println!(
                "{} Verified prerequisite 'JetBrainsMono Nerd Font' is installed.",
                accent("Dotengine")
            );
        }

        // Formulate request prompt
        let mut final_guidelines = custom_guidelines.unwrap_or_default();

        // Scan and load existing dotfiles config context to enable smart editing capabilities
        let mut existing_configs_context = String::new();
        let paths_to_probe = vec![
            ".config/hypr/hyprland.conf",
            ".config/waybar/config",
            ".config/waybar/style.css",
            ".config/rofi/config.rasi",
            ".config/swaync/config.json",
            ".config/swaync/style.css",
            ".config/dunst/dunstrc",
        ];

        for path_str in paths_to_probe {
            let path = std::path::Path::new(path_str);
            if let Ok(content) = self.system_manager.read_config_file(path).await {
                if !content.trim().is_empty() {
                    existing_configs_context
                        .push_str(&format!("\n--- FILE: {} ---\n{}\n", path_str, content));
                }
            }
        }

        if !existing_configs_context.is_empty() {
            let editing_rule = format!(
                "\n=== CURRENT ACTIVE CONFIGURATION CONTEXT ===\n\
                 The host system already has the following active configurations:\n{}\n\
                 SMART EDITING & RICE EXPANSION INSTRUCTIONS:\n\
                 1. If the user's prompt is an edit, tweak, or incremental modification of their existing setup (e.g. changing hotkeys/keybindings, adjusting gaps, changing font sizes, or swapping colors), you MUST ONLY modify the specific lines or files affected. Leave all other files and settings completely untouched!\n\
                 2. If the user asks to change or modify keybindings or hotkeys (e.g. changing the launcher binding to SUPER, R or adding a terminal bind), locate the specific 'bind = ...' rules in hyprland.conf and replace or append only those targeted binds. Do NOT modify or remove other unrelated hotkeys or window behaviors.\n\
                 3. If the user asks to expand their rice (e.g. adding a new module like CPU, memory, or battery to Waybar, or adding a new styling element to Rofi), you must insert the new module seamlessly into the active layout JSON/config and append the new CSS style classes to the bottom of the style file, preserving all existing layout and custom color theme configurations.\n\
                 4. Do NOT output or modify files that are not requested. If the user only asked to tweak Waybar, only output waybar/config and/or waybar/style.css. Do NOT output dunst, swaync, rofi, or hyprland.conf unless they are affected. Only return the files that actually contain changes.\n\
                 5. Do NOT discard their working configuration unless they explicitly request a complete redesign from scratch.",
                existing_configs_context
            );
            final_guidelines.push_str(&editing_rule);
        }

        if let Some(reference) = reference_spec {
            final_guidelines.push('\n');
            final_guidelines.push_str(&reference.to_guidelines());
            final_guidelines.push_str(
                "\nREFERENCE DESIGN RULES:\n\
                 1. Recreate the screenshot's visual language and startup behavior, not a pixel-perfect clone.\n\
                 2. If the screenshot implies a fuller setup, wire the necessary exec-once, portal, wallpaper, lockscreen, and tray behaviors so the rice feels complete.\n\
                 3. Use the extracted stack hints and completion notes to fill any missing pieces for a barebones Hyprland install.",
            );
        }

        // Append panel instructions
        if panel_util != "none" {
            let panel_rule = format!(
                "\nSTATUS PANEL / SHELL UTILITY: Use '{}'. Provide corresponding config files (e.g. '.config/waybar/config' and '.config/waybar/style.css' if using waybar, or JS/TS widget layouts under '.config/ags' if using ags, or QML widgets under '.config/quickshell' if using quickshell). Only write configs for '{}'.",
                panel_util, panel_util
            );
            final_guidelines.push_str(&panel_rule);
        }

        // Append launcher instructions
        if launcher_util != "none" {
            let launcher_rule = format!(
                "\nAPPLICATION LAUNCHER UTILITY: Use '{}'. Provide matching config files (such as '.config/rofi/config.rasi' if using rofi).",
                launcher_util
            );
            final_guidelines.push_str(&launcher_rule);
            if launcher_util == "rofi" {
                if let Some(bind) = rofi_bind {
                    let rofi_guideline = format!("\nCRITICAL KEYBINDING: You MUST map Rofi launcher to the exact shortcut: bind = {}, exec, rofi -show drun. Remove any other default Rofi launch bindings.", bind);
                    final_guidelines.push_str(&rofi_guideline);
                } else {
                    let rofi_guideline = "\nROFI KEYBINDING: Preserve any existing Rofi launcher keybinds unless the user explicitly requests a change. Do not add new Rofi keybinds if none exist.";
                    final_guidelines.push_str(rofi_guideline);
                }
            }
        }

        // Append notification center instructions
        if notification_util != "none" {
            let notif_rule = format!(
                "\nNOTIFICATION PANEL: Use '{}'. Provide matching config files (such as '.config/swaync/config.json' and '.config/swaync/style.css' if using swaync, or '.config/dunst/dunstrc' if using dunst). Launch it on startup in your hyprland.conf via 'exec-once = {}' (ags/dunst are autostarted by dbus/ags shell, swaync requires 'exec-once = swaync').",
                notification_util, notification_util
            );
            final_guidelines.push_str(&notif_rule);
        }

        if panel_util == "waybar" {
            let waybar_rule = "\nWAYBAR STARTUP: Ensure Hyprland launches Waybar on boot using 'exec-once = waybar' if it is not already present. If editing existing configs, preserve any custom Waybar launch commands while ensuring Waybar is started.";
            final_guidelines.push_str(waybar_rule);
        }

        // Append system applets launch instructions
        if enable_nm_applet {
            let applet_rule = "\nSYSTEM NETWORK MANAGER APPLET: You MUST launch the connection tray applet on startup in your hyprland.conf using: exec-once = nm-applet\n";
            final_guidelines.push_str(applet_rule);
        }
        if enable_blueman {
            let applet_rule = "\nSYSTEM BLUETOOTH MANAGER APPLET: You MUST launch the bluetooth tray applet on startup in your hyprland.conf using: exec-once = blueman-applet\n";
            final_guidelines.push_str(applet_rule);
        }

        // Append wallpaper utility instructions
        if wallpaper_util != "none" {
            let wallpaper_rule = format!(
                "\nWALLPAPER UTILITY: Use '{}'. Provide matching config files (such as '.config/hypr/hyprpaper.conf' with wallpaper paths if using hyprpaper).",
                wallpaper_util
            );
            final_guidelines.push_str(&wallpaper_rule);
            if !wallpaper_prompt.trim().is_empty() {
                let wall_desc = format!(
                    " Wallpaper design preference/aesthetic instructions: {}",
                    wallpaper_prompt
                );
                final_guidelines.push_str(&wall_desc);
            }
        }

        // Append lockscreen utility instructions
        if lockscreen_util != "none" {
            let lock_path = match lockscreen_util {
                "hyprlock" => ".config/hypr/hyprlock.conf",
                "swaylock" => ".config/swaylock/config",
                "waylock" => ".config/waylock/config",
                _ => ".config/hypr/hyprlock.conf",
            };
            let lockscreen_rule = format!(
                "\nLOCKSCREEN UTILITY: Use '{}'. Provide a highly matching, aesthetic layout configuration for the lockscreen (save config precisely to '{}' with active blurs, font styles, and color matching).",
                lockscreen_util, lock_path
            );
            final_guidelines.push_str(&lockscreen_rule);
            if !lockscreen_prompt.trim().is_empty() {
                let lock_desc = format!(
                    " Lockscreen design preference/aesthetic instructions: {}",
                    lockscreen_prompt
                );
                final_guidelines.push_str(&lock_desc);
            }
        }

        // Append strict blur syntax rules
        let blur_syntax_rule = "\nCRITICAL HYPRLAND DECORATION BLUR SYNTAX RULE:\n\
                               1. You MUST structure all blur parameters inside nested 'blur { ... }' blocks inside 'decoration { ... }'.\n\
                               2. NEVER define blur properties directly within 'decoration { ... }' (e.g. do NOT use 'blur = true', 'blur_size = ...', or 'decoration:blur'). Doing so will trigger critical parsing errors on reload.";
        final_guidelines.push_str(blur_syntax_rule);

        // Append critical invalid-key guardrails
        let invalid_key_rule = "\nCRITICAL INVALID KEY RULES:\n\
                                1. NEVER add a 'waybar:' section or key inside hyprland.conf/hyprland.lua. Waybar is a separate process and must be started with 'exec-once = waybar'.\n\
                                2. NEVER emit tokens like 'type ignorezero' in Hyprland configs. Use only documented keys for the detected Hyprland version.";
        final_guidelines.push_str(invalid_key_rule);

        // Append icon font / unicode block guidelines
        let icon_rule = "\nUNICODE ICON / FONT GLYPH RULES:\n\
                          1. Declare fallback font families in all Rofi, Waybar, and AGS CSS configurations to support standard FontAwesome and Nerd Font styles:\n\
                             font-family: \"JetBrainsMono Nerd Font\", \"Font Awesome 6 Free\", \"FontAwesome\", sans-serif;\n\
                          2. Utilize standard, widely compatible unicode glyphs/icons for Waybar modules (e.g. standard battery/wifi/sound icons) to avoid raw system-unsupported unicode tofu blocks.";
        final_guidelines.push_str(icon_rule);

        // Append multimedia & function key (Fn key) mapping instructions
        let multimedia_bind_rule = "\nMULTIMEDIA & FUNCTION KEY MAPPING RULES:\n\
                                    When creating or modifying '.config/hypr/hyprland.conf', you MUST automatically include standard bindings for function keys (Fn keys) to support audio, brightness, and media control. Use the following robust rules:\n\
                                    1. Volume Control (use binde for repeat, and wire to wpctl or pactl):\n\
                                       binde = , XF86AudioRaiseVolume, exec, wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%+\n\
                                       binde = , XF86AudioLowerVolume, exec, wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-\n\
                                       bind = , XF86AudioMute, exec, wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle\n\
                                       bind = , XF86AudioMicMute, exec, wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle\n\
                                    2. Backlight/Brightness Control (use binde for repeat, and wire to brightnessctl or light):\n\
                                       binde = , XF86MonBrightnessUp, exec, brightnessctl set 5%+\n\
                                       binde = , XF86MonBrightnessDown, exec, brightnessctl set 5%-\n\
                                    3. Media Player Control (wire to playerctl):\n\
                                       bind = , XF86AudioPlay, exec, playerctl play-pause\n\
                                       bind = , XF86AudioNext, exec, playerctl next\n\
                                       bind = , XF86AudioPrev, exec, playerctl previous\n\
                                    Ensure these are clearly commented and added to the binds section.";
        final_guidelines.push_str(multimedia_bind_rule);

        let mut prompt = UserPrompt::new(raw_prompt).with_guidelines(final_guidelines);
        if !image_payloads.is_empty() {
            prompt = prompt.with_images(image_payloads);
        }

        // Hit LLM Service
        println!();
        let generated_configs = self
            .generate_with_validation(&prompt, &system_context, design_rules_content)
            .await?;
        println!(
            "{} Synthesis completed. Received {} configuration files.",
            accent("Dotengine"),
            generated_configs.len()
        );

        if !self
            .system_manager
            .confirm_config_changes(&generated_configs)
            .await?
        {
            return Err("User declined generated configuration changes".into());
        }

        // Write approved files within the supported desktop configuration scope.
        for config in &generated_configs {
            println!(
                "    - Applying config file: '~/{}' ({} bytes)",
                config.relative_path.display(),
                config.content.len()
            );
            self.system_manager.write_config_file(config).await?;
        }

        // Execute reload / verification
        let wallpaper_query = reference_spec.and_then(|spec| spec.wallpaper_description.as_deref());
        match activity(
            "Validating and reloading Hyprland",
            self.system_manager.verify_and_reload(&generated_configs, wallpaper_query),
        )
        .await
        {
            Ok(_) => {
                println!(
                    "{} Configuration reload completed successfully.",
                    accent("Dotengine")
                );
                Ok(generated_configs)
            }
            Err(error_payload) => {
                println!(
                    "\n{} Configuration reload failed. Starting self-healing recovery.",
                    accent("Dotengine")
                );
                println!("    Faulting Command: {}", error_payload.command);
                println!("    Stderr log details:\n    {}", error_payload.stderr);

                // Trigger Self-Healing recovery loop
                let healed_configs = self
                    .healing_workflow
                    .execute(error_payload, design_rules_content, 1)
                    .await?;
                Ok(healed_configs)
            }
        }
    }

    async fn generate_with_validation(
        &self,
        prompt: &UserPrompt,
        system_context: &crate::domain::SystemContext,
        design_rules_content: &str,
    ) -> Result<Vec<ConfigFile>, Box<dyn std::error::Error + Send + Sync>> {
        const MAX_RETRIES: usize = 3;
        let mut retry_prompt = prompt.clone();

        for attempt in 1..=MAX_RETRIES {
            let generated_configs = activity(
                "Generating desktop configuration",
                self.ai_service.generate_config(
                    &retry_prompt,
                    system_context,
                    design_rules_content,
                ),
            )
            .await?;

            if let Err(errors) = validate_generated_configs(&generated_configs) {
                println!(
                    "{} Invalid configuration detected (attempt {}/{}).",
                    accent("Dotengine"),
                    attempt,
                    MAX_RETRIES
                );
                for error in &errors {
                    println!("    - {}", error);
                }

                let mut guidelines = retry_prompt.custom_guidelines.clone().unwrap_or_default();
                guidelines.push_str("\n\nCRITICAL REGENERATION RULES:\n");
                for error in errors {
                    guidelines.push_str(&format!(
                        "- Fix this error and do not reintroduce it: {}\n",
                        error
                    ));
                }
                retry_prompt.custom_guidelines = Some(guidelines);
                continue;
            }

            return Ok(generated_configs);
        }

        Err("Failed to generate valid configuration after multiple attempts.".into())
    }
}

fn validate_generated_configs(configs: &[ConfigFile]) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for config in configs {
        if config
            .relative_path
            .to_string_lossy()
            .ends_with("/hyprland.conf")
        {
            let lower = config.content.to_lowercase();
            if lower.contains("waybar:") {
                errors.push("hyprland.conf: invalid key 'waybar:'".to_string());
            }
            if lower.contains("type ignorezero") {
                errors.push("hyprland.conf: invalid token 'type ignorezero'".to_string());
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn extract_path_from_prompt(prompt: &str) -> Option<String> {
    let prompt = prompt.replace('\'', "\"");
    if let Some(start) = prompt.find('"') {
        if let Some(end) = prompt[start + 1..].find('"') {
            return Some(prompt[start + 1..start + 1 + end].trim().to_string());
        }
    }
    for word in prompt.split_whitespace() {
        if word.starts_with('/') || word.starts_with('~') || word.contains('.') && word.contains('/') {
            return Some(word.trim().to_string());
        }
    }
    None
}

fn normalize_path(path: &str) -> std::path::PathBuf {
    let path_str = if path.starts_with('~') {
        if let Some(home) = std::env::var_os("HOME") {
            path.replacen('~', &home.to_string_lossy(), 1)
        } else {
            path.to_string()
        }
    } else {
        path.to_string()
    };
    std::path::PathBuf::from(path_str)
}
