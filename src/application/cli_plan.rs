use crate::application::normalize_stack_values;
use crate::domain::DesignTemplate;
use clap::ValueEnum;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptIntent {
    Action,
    Edit,
    Redesign,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentStack {
    pub panel: String,
    pub launcher: String,
    pub wallpaper: String,
    pub lockscreen: String,
    pub notification: String,
}

impl ComponentStack {
    pub fn new(
        panel: impl Into<String>,
        launcher: impl Into<String>,
        wallpaper: impl Into<String>,
        lockscreen: impl Into<String>,
        notification: impl Into<String>,
    ) -> Self {
        let (panel, launcher, wallpaper, lockscreen, notification) = normalize_stack_values(
            panel.into(),
            launcher.into(),
            wallpaper.into(),
            lockscreen.into(),
            notification.into(),
        );
        Self {
            panel,
            launcher,
            wallpaper,
            lockscreen,
            notification,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum BackupMode {
    Prompt,
    Auto,
    Off,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Default)]
pub struct ExistingStack {
    pub panel: Option<String>,
    pub launcher: Option<String>,
    pub wallpaper: Option<String>,
    pub lockscreen: Option<String>,
    pub notification: Option<String>,
    pub has_any: bool,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunPlan {
    pub prompt_intent: PromptIntent,
    pub stack: ComponentStack,
    pub template_index: Option<usize>,
    pub template_name: Option<String>,
    pub interactive: bool,
    pub dry_run: bool,
    pub non_interactive: bool,
    pub backup_mode: BackupMode,
    pub auto_install: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptTriggers {
    pub rofi_bind: bool,
    pub wallpaper_prompt: bool,
    pub lockscreen_prompt: bool,
    pub hyprland_version_prompt: bool,
}

pub fn validate_template_index(template_index: Option<usize>) -> Result<Option<usize>, String> {
    match template_index {
        Some(idx) if idx < DesignTemplate::get_predefined_library().len() => Ok(Some(idx)),
        Some(idx) => Err(format!(
            "Invalid template index {}. Valid indices are 0..{}.",
            idx,
            DesignTemplate::get_predefined_library()
                .len()
                .saturating_sub(1)
        )),
        None => Ok(None),
    }
}

pub fn prompt_suggests_redesign(prompt: &str) -> bool {
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

pub fn prompt_mentions_keybinds(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let keywords = [
        "keybind",
        "key binding",
        "shortcut",
        "hotkey",
        "bind =",
        "binds",
    ];
    keywords.iter().any(|kw| lower.contains(kw))
}

pub fn prompt_mentions_wallpaper(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let keywords = ["wallpaper", "background", "wall paper", "swww", "hyprpaper"];
    keywords.iter().any(|kw| lower.contains(kw))
}

pub fn prompt_mentions_lockscreen(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let keywords = [
        "lockscreen",
        "lock screen",
        "hyprlock",
        "swaylock",
        "waylock",
    ];
    keywords.iter().any(|kw| lower.contains(kw))
}

pub fn prompt_is_direct_action(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    (lower.contains("set") || lower.contains("change") || lower.contains("use"))
        && (lower.contains("wallpaper") || lower.contains("background"))
        && (lower.contains('/') || lower.contains('.') || lower.contains("wallhaven") || lower.contains("web"))
}

pub fn classify_intent(prompt: &str, has_existing_setup: bool) -> PromptIntent {
    if prompt_is_direct_action(prompt) {
        PromptIntent::Action
    } else if prompt_suggests_redesign(prompt) {
        PromptIntent::Redesign
    } else if has_existing_setup {
        PromptIntent::Edit
    } else {
        PromptIntent::Redesign
    }
}

pub fn stack_override_from_prompt(prompt: &str) -> ComponentStack {
    let lower = prompt.to_lowercase();
    let mut panel = None;
    let mut launcher = None;
    let mut wallpaper = None;
    let mut lockscreen = None;
    let mut notification = None;

    if lower.contains("waybar") {
        panel = Some("waybar");
    } else if lower.contains("ags") || lower.contains("aylur") {
        panel = Some("ags");
    } else if lower.contains("quick-shell") || lower.contains("quickshell") {
        panel = Some("quick-shell");
    } else if lower.contains("no panel") || lower.contains("without panel") {
        panel = Some("none");
    }

    if lower.contains("rofi") {
        launcher = Some("rofi");
    } else if lower.contains("no launcher") || lower.contains("without launcher") {
        launcher = Some("none");
    }

    if lower.contains("hyprpaper") {
        wallpaper = Some("hyprpaper");
    } else if lower.contains("no wallpaper") || lower.contains("without wallpaper") {
        wallpaper = Some("none");
    }

    if lower.contains("hyprlock") {
        lockscreen = Some("hyprlock");
    } else if lower.contains("swaylock") {
        lockscreen = Some("swaylock");
    } else if lower.contains("waylock") {
        lockscreen = Some("waylock");
    } else if lower.contains("no lockscreen") || lower.contains("without lockscreen") {
        lockscreen = Some("none");
    }

    if lower.contains("swaync") {
        notification = Some("swaync");
    } else if lower.contains("dunst") {
        notification = Some("dunst");
    } else if lower.contains("no notification") || lower.contains("without notifications") {
        notification = Some("none");
    }

    ComponentStack::new(
        panel.unwrap_or(""),
        launcher.unwrap_or(""),
        wallpaper.unwrap_or(""),
        lockscreen.unwrap_or(""),
        notification.unwrap_or(""),
    )
}

pub fn detect_existing_stack(home_dir: &Path) -> ExistingStack {
    let mut detection = ExistingStack::default();
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

pub fn heuristic_stack_for_prompt(
    prompt: &str,
    template_idx: Option<usize>,
) -> Result<ComponentStack, String> {
    let predefined_templates = DesignTemplate::get_predefined_library();
    let resolved_idx = validate_template_index(template_idx)?;

    if let Some(idx) = resolved_idx {
        let template_name = &predefined_templates[idx].name;
        let stack = if template_name.contains("Glass") {
            ComponentStack::new("ags", "rofi", "hyprpaper", "hyprlock", "ags")
        } else if template_name.contains("Nord") {
            ComponentStack::new("waybar", "rofi", "hyprpaper", "hyprlock", "swaync")
        } else if template_name.contains("Cyber") {
            ComponentStack::new("waybar", "rofi", "hyprpaper", "hyprlock", "swaync")
        } else {
            ComponentStack::new("waybar", "rofi", "hyprpaper", "hyprlock", "swaync")
        };
        return Ok(stack);
    }

    let lower = prompt.to_lowercase();
    if lower.contains("blur")
        || lower.contains("glass")
        || lower.contains("widget")
        || lower.contains("glow")
    {
        Ok(ComponentStack::new(
            "ags",
            "rofi",
            "hyprpaper",
            "hyprlock",
            "ags",
        ))
    } else if lower.contains("minimal") || lower.contains("nord") || lower.contains("arctic") {
        Ok(ComponentStack::new(
            "waybar",
            "rofi",
            "hyprpaper",
            "hyprlock",
            "swaync",
        ))
    } else {
        Ok(ComponentStack::new(
            "waybar",
            "rofi",
            "hyprpaper",
            "hyprlock",
            "swaync",
        ))
    }
}

pub fn resolve_component_stack(
    prompt: &str,
    template_idx: Option<usize>,
    existing_stack: &ExistingStack,
) -> Result<(ComponentStack, Option<String>), String> {
    let prompt_override = stack_override_from_prompt(prompt);
    let heuristic = heuristic_stack_for_prompt(prompt, template_idx)?;
    let stack = ComponentStack {
        panel: if !prompt_override.panel.is_empty() {
            prompt_override.panel
        } else {
            existing_stack
                .panel
                .clone()
                .unwrap_or_else(|| heuristic.panel.clone())
        },
        launcher: if !prompt_override.launcher.is_empty() {
            prompt_override.launcher
        } else {
            existing_stack
                .launcher
                .clone()
                .unwrap_or_else(|| heuristic.launcher.clone())
        },
        wallpaper: if !prompt_override.wallpaper.is_empty() {
            prompt_override.wallpaper
        } else {
            existing_stack
                .wallpaper
                .clone()
                .unwrap_or_else(|| heuristic.wallpaper.clone())
        },
        lockscreen: if !prompt_override.lockscreen.is_empty() {
            prompt_override.lockscreen
        } else {
            existing_stack
                .lockscreen
                .clone()
                .unwrap_or_else(|| heuristic.lockscreen.clone())
        },
        notification: if !prompt_override.notification.is_empty() {
            prompt_override.notification
        } else {
            existing_stack
                .notification
                .clone()
                .unwrap_or_else(|| heuristic.notification.clone())
        },
    };

    let template_name = template_idx.and_then(|idx| {
        DesignTemplate::get_predefined_library()
            .get(idx)
            .map(|template| template.name.clone())
    });

    Ok((stack, template_name))
}

pub fn should_prompt_rofi_bind(
    launcher: &str,
    rofi_bind_is_missing: bool,
    interactive: bool,
    is_new_user: bool,
    intent: PromptIntent,
    prompt: &str,
) -> bool {
    launcher == "rofi"
        && rofi_bind_is_missing
        && interactive
        && (is_new_user || intent == PromptIntent::Redesign || prompt_mentions_keybinds(prompt))
}

pub fn should_prompt_component_text(
    component: &str,
    interactive: bool,
    is_new_user: bool,
    prompt_matches: bool,
) -> bool {
    component != "none" && interactive && (is_new_user || prompt_matches)
}

pub fn should_prompt_hyprland_version(interactive: bool) -> bool {
    interactive
}

pub fn build_run_plan(
    prompt: String,
    template_index: Option<usize>,
    existing_stack: ExistingStack,
    interactive: bool,
    dry_run: bool,
    non_interactive: bool,
    backup_mode: BackupMode,
    auto_install: bool,
) -> Result<RunPlan, String> {
    let prompt_intent = classify_intent(&prompt, existing_stack.has_any);
    let (stack, template_name) = resolve_component_stack(&prompt, template_index, &existing_stack)?;

    Ok(RunPlan {
        prompt_intent,
        stack,
        template_index,
        template_name,
        interactive,
        dry_run,
        non_interactive,
        backup_mode,
        auto_install,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_redesign_prompts() {
        assert_eq!(
            classify_intent("redesign the whole desktop", true),
            PromptIntent::Redesign
        );
        assert_eq!(classify_intent("tweak gaps", true), PromptIntent::Edit);
        assert_eq!(classify_intent("tweak gaps", false), PromptIntent::Redesign);
        assert_eq!(
            classify_intent("set ~/Pictures/wall.png as wallpaper", true),
            PromptIntent::Action
        );
        assert_eq!(
            classify_intent("change wallpaper to /usr/share/bg.jpg", false),
            PromptIntent::Action
        );
    }

    #[test]
    fn detects_stack_overrides() {
        let override_stack = stack_override_from_prompt("use ags and no notification");
        assert_eq!(override_stack.panel, "ags");
        assert_eq!(override_stack.notification, "none");
    }

    #[test]
    fn normalizes_component_stack_values() {
        let stack = ComponentStack::new(
            "WayBar",
            "ROFI",
            "Hyprpaper2 (or Hyprpaper with custom gradient asset/script/static image matching the pastel gradient background in the screenshot.)",
            "HyprLock",
            "SwayNC",
        );

        assert_eq!(stack.panel, "waybar");
        assert_eq!(stack.launcher, "rofi");
        assert_eq!(stack.wallpaper, "hyprpaper");
        assert_eq!(stack.lockscreen, "hyprlock");
        assert_eq!(stack.notification, "swaync");
    }

    #[test]
    fn rejects_invalid_template_indices() {
        assert!(validate_template_index(Some(999)).is_err());
    }

    #[test]
    fn suppresses_prompting_when_not_interactive() {
        assert!(!should_prompt_rofi_bind(
            "rofi",
            true,
            false,
            true,
            PromptIntent::Redesign,
            "change shortcuts"
        ));
    }

    #[test]
    fn resolves_existing_stack_and_heuristics() {
        let existing = ExistingStack {
            panel: Some("waybar".to_string()),
            launcher: Some("rofi".to_string()),
            wallpaper: None,
            lockscreen: None,
            notification: None,
            has_any: true,
        };

        let (stack, _) = resolve_component_stack("minimal desktop", None, &existing).unwrap();
        assert_eq!(stack.panel, "waybar");
        assert_eq!(stack.launcher, "rofi");
    }

    #[test]
    fn detects_existing_stack_from_paths() {
        let home = PathBuf::from("/tmp/dotengine-home");
        let _ = detect_existing_stack(&home);
    }
}
